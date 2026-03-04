/// E2E 集成测试：CLI 完整链路
///
/// 测试场景：
/// 1. init 扫描文件并创建 blocks
/// 2. init → register → editor + grants 写入
/// 3. scan 增量同步新文件
/// 4. agent 创建+写入 block
/// 5. grant/revoke 权限控制
/// 6. 细粒度 block 权限
/// 7. resolve_block_id name/id 双模
/// 8. 模板解析含 grants 字段
use elfiee_lib::cli;
use elfiee_lib::models::Command;
use elfiee_lib::services::project;
use elfiee_lib::state::AppState;
use serde_json::json;
use std::collections::HashSet;
use tempfile::TempDir;

/// 创建临时项目目录，含若干测试文件
fn create_test_project() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // 创建源文件
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn hello() {}").unwrap();
    std::fs::write(root.join("README.md"), "# Test Project").unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

    tmp
}

// ============================================================================
// Test 1: init 扫描文件并创建 blocks
// ============================================================================

#[tokio::test]
async fn test_init_scans_files() {
    let tmp = create_test_project();
    let project = tmp.path().to_str().unwrap();

    cli::init::run(project).await.expect("init failed");

    // 验证 .elf/ 存在
    assert!(tmp.path().join(".elf").exists());

    // 打开项目验证 blocks
    let state = AppState::new();
    let file_id = project::open_project(project, &state)
        .await
        .expect("open failed");
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    let blocks = handle.get_all_blocks().await;

    // 应该有 blocks（至少 src/main.rs, src/lib.rs, README.md, Cargo.toml）
    assert!(
        blocks.len() >= 4,
        "Expected at least 4 blocks, got {}",
        blocks.len()
    );

    // block name 应该是相对路径
    let names: HashSet<String> = blocks.values().map(|b| b.name.clone()).collect();
    assert!(names.contains("src/main.rs"), "Missing src/main.rs block");
    assert!(names.contains("src/lib.rs"), "Missing src/lib.rs block");
    assert!(names.contains("README.md"), "Missing README.md block");
}

// ============================================================================
// Test 2: init → register → editor + grants
// ============================================================================

#[tokio::test]
async fn test_init_register_flow() {
    let tmp = create_test_project();
    let project = tmp.path().to_str().unwrap();

    // init
    cli::init::run(project).await.expect("init failed");

    // register
    let config_dir = tmp.path().join(".test-agent-config");
    let editor_id = cli::register::run(
        "openclaw",
        Some("test-agent"),
        Some(config_dir.to_str().unwrap()),
        project,
        47200,
    )
    .await
    .expect("register failed");

    assert!(editor_id.starts_with("openclaw-"));

    // 验证 MCP server 写入 .mcp.json（config_dir 的上级目录）
    let mcp_json_path = config_dir.parent().unwrap().join(".mcp.json");
    assert!(mcp_json_path.exists(), ".mcp.json should exist");
    let mcp_content = std::fs::read_to_string(&mcp_json_path).unwrap();
    let mcp_json: serde_json::Value = serde_json::from_str(&mcp_content).unwrap();
    assert_eq!(
        mcp_json["mcpServers"]["elfiee"]["url"],
        "http://localhost:47200/sse"
    );

    // 验证 env 和 permissions 写入 settings.local.json
    let settings_path = config_dir.join("settings.local.json");
    assert!(settings_path.exists());
    let content = std::fs::read_to_string(&settings_path).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(settings["env"]["ELFIEE_EDITOR_ID"], editor_id);
    // 验证 ELFIEE_PROJECT 注入
    assert!(settings["env"]["ELFIEE_PROJECT"].is_string());
    // 验证 permissions 注入
    assert!(settings["permissions"]["allow"].is_array());

    // 验证 editor 和 grants 在 eventstore 中
    let state = AppState::new();
    let file_id = project::open_project(project, &state)
        .await
        .expect("open failed");
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    let editors = handle.get_all_editors().await;
    assert!(
        editors.contains_key(&editor_id),
        "Editor {} not found",
        editor_id
    );

    // 验证有 grants
    let grants = handle.get_editor_grants(editor_id.clone()).await;
    assert!(!grants.is_empty(), "Editor should have grants");

    // 验证 Skill 注入
    let skill_path = config_dir.join("skills/elfiee/SKILL.md");
    assert!(skill_path.exists(), "SKILL.md not injected");
}

// ============================================================================
// Test 3: scan 增量同步
// ============================================================================

#[tokio::test]
async fn test_scan_incremental() {
    let tmp = create_test_project();
    let project = tmp.path().to_str().unwrap();

    // init（创建初始 blocks）
    cli::init::run(project).await.expect("init failed");

    // 获取初始 block 数量
    let state = AppState::new();
    let file_id = project::open_project(project, &state).await.unwrap();
    let handle = state.engine_manager.get_engine(&file_id).unwrap();
    let initial_blocks = handle.get_all_blocks().await;
    let initial_count = initial_blocks.len();
    drop(state);

    // 外部新增文件
    std::fs::write(tmp.path().join("src/new_module.rs"), "pub fn new() {}").unwrap();
    std::fs::write(tmp.path().join("config.toml"), "[settings]").unwrap();

    // scan
    cli::scan::run(project, None).await.expect("scan failed");

    // 验证新 blocks 被创建
    let state2 = AppState::new();
    let file_id2 = project::open_project(project, &state2).await.unwrap();
    let handle2 = state2.engine_manager.get_engine(&file_id2).unwrap();
    let after_blocks = handle2.get_all_blocks().await;

    assert!(
        after_blocks.len() > initial_count,
        "Expected more blocks after scan: {} > {}",
        after_blocks.len(),
        initial_count
    );

    let names: HashSet<String> = after_blocks.values().map(|b| b.name.clone()).collect();
    assert!(
        names.contains("src/new_module.rs"),
        "Missing new_module.rs block"
    );
    assert!(names.contains("config.toml"), "Missing config.toml block");
}

// ============================================================================
// Test 4: agent 创建+写入 block
// ============================================================================

#[tokio::test]
async fn test_agent_create_write_block() {
    let tmp = create_test_project();
    let project = tmp.path().to_str().unwrap();

    cli::init::run(project).await.unwrap();

    // register agent
    let config_dir = tmp.path().join(".agent-config");
    let editor_id = cli::register::run(
        "openclaw",
        None,
        Some(config_dir.to_str().unwrap()),
        project,
        47200,
    )
    .await
    .unwrap();

    // 打开项目，agent 创建新 block
    let state = AppState::new();
    let file_id = project::open_project(project, &state).await.unwrap();
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    let cmd = Command::new(
        editor_id.clone(),
        "core.create".to_string(),
        "".to_string(),
        json!({
            "name": "agent-created.md",
            "block_type": "document",
            "format": "md"
        }),
    );
    let events = handle
        .process_command(cmd)
        .await
        .expect("Agent should be able to create blocks");

    let block_id = events[0].entity.clone();

    // agent 写入内容
    let write_cmd = Command::new(
        editor_id,
        "document.write".to_string(),
        block_id.clone(),
        json!({ "content": "# Agent Created\nHello from agent!" }),
    );
    handle
        .process_command(write_cmd)
        .await
        .expect("Agent should be able to write documents");

    // 验证内容
    let block = handle.get_block(block_id).await.unwrap();
    assert_eq!(
        block.contents["content"],
        "# Agent Created\nHello from agent!"
    );
}

// ============================================================================
// Test 5: grant → revoke → agent 被拒
// ============================================================================

#[tokio::test]
async fn test_grant_revoke_permission_flow() {
    let tmp = create_test_project();
    let project = tmp.path().to_str().unwrap();

    cli::init::run(project).await.unwrap();

    // register agent
    let config_dir = tmp.path().join(".agent-config");
    let editor_id = cli::register::run(
        "openclaw",
        None,
        Some(config_dir.to_str().unwrap()),
        project,
        47200,
    )
    .await
    .unwrap();

    // 打开项目
    let state = AppState::new();
    let file_id = project::open_project(project, &state).await.unwrap();
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    // agent 应该能创建 block（有 core.create grant）
    let cmd = Command::new(
        editor_id.clone(),
        "core.create".to_string(),
        "".to_string(),
        json!({ "name": "test.md", "block_type": "document", "format": "md" }),
    );
    let events = handle.process_command(cmd).await.unwrap();
    let block_id = events[0].entity.clone();

    // agent 能写自己创建的 block（owner）
    let write_cmd = Command::new(
        editor_id.clone(),
        "document.write".to_string(),
        block_id.clone(),
        json!({ "content": "initial" }),
    );
    assert!(handle.process_command(write_cmd).await.is_ok());

    // System revoke agent 的 document.write wildcard
    let system_id =
        elfiee_lib::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());
    let revoke_cmd = Command::new(
        system_id.clone(),
        "core.revoke".to_string(),
        "*".to_string(),
        json!({
            "target_editor": editor_id,
            "capability": "document.write",
            "target_block": "*"
        }),
    );
    handle.process_command(revoke_cmd).await.unwrap();

    // Agent 仍能写自己的 block（因为是 owner）
    let write_cmd2 = Command::new(
        editor_id.clone(),
        "document.write".to_string(),
        block_id.clone(),
        json!({ "content": "still owner" }),
    );
    assert!(
        handle.process_command(write_cmd2).await.is_ok(),
        "Owner should still be able to write their own block"
    );
}

// ============================================================================
// Test 6: 细粒度 block 权限
// ============================================================================

#[tokio::test]
async fn test_fine_grained_block_permission() {
    let tmp = create_test_project();
    let project = tmp.path().to_str().unwrap();

    cli::init::run(project).await.unwrap();

    // 打开项目
    let state = AppState::new();
    let file_id = project::open_project(project, &state).await.unwrap();
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    let system_id =
        elfiee_lib::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    // 创建 editor bob（无任何 grants）
    let create_editor_cmd = Command::new(
        system_id.clone(),
        "editor.create".to_string(),
        "".to_string(),
        json!({ "name": "Bob", "editor_type": "Bot" }),
    );
    let events = handle.process_command(create_editor_cmd).await.unwrap();
    let bob_id = events[0].entity.clone();

    // 系统创建两个 block
    let cmd_a = Command::new(
        system_id.clone(),
        "core.create".to_string(),
        "".to_string(),
        json!({ "name": "block_a.md", "block_type": "document", "format": "md" }),
    );
    let events_a = handle.process_command(cmd_a).await.unwrap();
    let block_a_id = events_a[0].entity.clone();

    let cmd_b = Command::new(
        system_id.clone(),
        "core.create".to_string(),
        "".to_string(),
        json!({ "name": "block_b.md", "block_type": "document", "format": "md" }),
    );
    let events_b = handle.process_command(cmd_b).await.unwrap();
    let block_b_id = events_b[0].entity.clone();

    // 只 grant bob document.write on block_a
    let grant_cmd = Command::new(
        system_id.clone(),
        "core.grant".to_string(),
        block_a_id.clone(),
        json!({
            "target_editor": bob_id,
            "capability": "document.write",
            "target_block": block_a_id
        }),
    );
    handle.process_command(grant_cmd).await.unwrap();

    // Bob 写 block_a → 成功
    let write_a = Command::new(
        bob_id.clone(),
        "document.write".to_string(),
        block_a_id.clone(),
        json!({ "content": "bob writes a" }),
    );
    assert!(
        handle.process_command(write_a).await.is_ok(),
        "Bob should be able to write block_a"
    );

    // Bob 写 block_b → 拒绝
    let write_b = Command::new(
        bob_id.clone(),
        "document.write".to_string(),
        block_b_id.clone(),
        json!({ "content": "bob writes b" }),
    );
    let result = handle.process_command(write_b).await;
    assert!(result.is_err(), "Bob should NOT be able to write block_b");
    assert!(result.unwrap_err().contains("Authorization failed"));
}

// ============================================================================
// Test 7: resolve_block_id name/id 双模
// ============================================================================

#[tokio::test]
async fn test_resolve_block_by_name_and_id() {
    let tmp = create_test_project();
    let project = tmp.path().to_str().unwrap();

    cli::init::run(project).await.unwrap();

    let state = AppState::new();
    let file_id = project::open_project(project, &state).await.unwrap();
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    // 找到 src/main.rs 的 block
    let blocks = handle.get_all_blocks().await;
    let main_block = blocks
        .values()
        .find(|b| b.name == "src/main.rs")
        .expect("src/main.rs block should exist");
    let main_id = main_block.block_id.clone();

    // 按 name 解析
    let resolved_by_name = cli::resolve::resolve_block_id(&handle, "src/main.rs")
        .await
        .unwrap();
    assert_eq!(resolved_by_name, main_id);

    // 按 id 解析
    let resolved_by_id = cli::resolve::resolve_block_id(&handle, &main_id)
        .await
        .unwrap();
    assert_eq!(resolved_by_id, main_id);

    // Wildcard 直通
    let resolved_wildcard = cli::resolve::resolve_block_id(&handle, "*").await.unwrap();
    assert_eq!(resolved_wildcard, "*");

    // 不存在的 → 报错
    let result = cli::resolve::resolve_block_id(&handle, "nonexistent.rs").await;
    assert!(result.is_err());
}

// ============================================================================
// Test 8: 模板解析含 grants 字段
// ============================================================================

#[test]
fn test_template_with_fine_grained_grants() {
    let toml_content = r#"
[socialware]
name = "Fine Grained Test"
namespace = "fg"
description = "Test fine-grained permissions"

[[roles]]
id = "writer"
agent_type = "openclaw"
capabilities = ["session.append", "session.read"]
grants = [
    { capability = "document.write", block = "src/main.rs" },
    { capability = "document.read", block = "*" },
]

[[roles]]
id = "reviewer"
agent_type = "openclaw"
capabilities = ["document.read", "session.append"]
"#;

    let template: cli::run::SocialwareTemplate =
        toml::from_str(toml_content).expect("Failed to parse template");

    assert_eq!(template.roles.len(), 2);

    // writer 有 grants
    let writer = &template.roles[0];
    assert_eq!(writer.id, "writer");
    assert_eq!(writer.capabilities.len(), 2);
    assert_eq!(writer.grants.len(), 2);
    assert_eq!(writer.grants[0].capability, "document.write");
    assert_eq!(writer.grants[0].block, "src/main.rs");
    assert_eq!(writer.grants[1].block, "*");

    // reviewer 没有 grants
    let reviewer = &template.roles[1];
    assert_eq!(reviewer.id, "reviewer");
    assert!(reviewer.grants.is_empty());
}
