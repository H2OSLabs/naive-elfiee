/// 集成测试：project.rs 共享项目管理逻辑
///
/// 测试场景：
/// 1. seed_bootstrap_events — 新项目写入 bootstrap events
/// 2. seed_bootstrap_events — 已有 events 时 no-op
/// 3. open_project — 新建项目目录、初始化、spawn engine
/// 4. open_project — 重复打开返回已有 file_id
/// 5. close_project — 关闭后 engine 和 file entry 被清理
/// 6. close_project_by_id — 按 file_id 关闭
/// 7. close_project — 关闭未打开的项目报错
use elfiee_lib::elf_project::ElfProject;
use elfiee_lib::engine::EventStore;
use elfiee_lib::services::project;
use elfiee_lib::state::AppState;

/// 辅助函数：创建临时目录并初始化 .elf/ 项目
async fn init_temp_project(base: &std::path::Path, name: &str) -> std::path::PathBuf {
    let project_path = base.join(name);
    std::fs::create_dir_all(&project_path).unwrap();
    ElfProject::init(&project_path).await.unwrap();
    project_path
}

// ============================================================================
// seed_bootstrap_events 测试
// ============================================================================

#[tokio::test]
async fn test_seed_bootstrap_events_new_project() {
    let event_pool = EventStore::create(":memory:")
        .await
        .expect("Failed to create in-memory event store");

    // 新项目应该成功写入 bootstrap events
    project::seed_bootstrap_events(&event_pool)
        .await
        .expect("Failed to seed bootstrap events");

    let events = EventStore::get_all_events(&event_pool.pool)
        .await
        .expect("Failed to get events");

    // 至少有 1 个 editor.create + 多个 core.grant
    assert!(events.len() > 1, "Should have bootstrap events");

    // 第一个事件应该是 editor.create
    assert!(
        events[0].attribute.contains("editor.create"),
        "First event should be editor.create"
    );

    // 后续事件应该是 core.grant
    for event in &events[1..] {
        assert!(
            event.attribute.contains("core.grant"),
            "Non-first events should be core.grant, got: {}",
            event.attribute
        );
    }
}

#[tokio::test]
async fn test_seed_bootstrap_events_idempotent() {
    let event_pool = EventStore::create(":memory:")
        .await
        .expect("Failed to create in-memory event store");

    // 第一次 seed
    project::seed_bootstrap_events(&event_pool)
        .await
        .expect("First seed failed");

    let events_after_first = EventStore::get_all_events(&event_pool.pool)
        .await
        .expect("Failed to get events");

    // 第二次 seed — 应该 no-op
    project::seed_bootstrap_events(&event_pool)
        .await
        .expect("Second seed should be no-op");

    let events_after_second = EventStore::get_all_events(&event_pool.pool)
        .await
        .expect("Failed to get events");

    assert_eq!(
        events_after_first.len(),
        events_after_second.len(),
        "Second seed should not add more events"
    );
}

// ============================================================================
// open_project / close_project 测试
// ============================================================================

#[tokio::test]
async fn test_open_project_requires_init() {
    let state = AppState::new();
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_path = tmp_dir.path().join("test_project");
    std::fs::create_dir_all(&project_path).unwrap();
    let path_str = project_path.to_str().unwrap();

    // 没有 .elf/ 应该报错
    let result = project::open_project(path_str, &state).await;
    assert!(result.is_err(), "Should fail without .elf/");
    assert!(result.unwrap_err().contains("elf init"));
}

#[tokio::test]
async fn test_open_project_after_init() {
    let state = AppState::new();
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_path = init_temp_project(tmp_dir.path(), "test_project").await;
    let path_str = project_path.to_str().unwrap();

    let file_id = project::open_project(path_str, &state)
        .await
        .expect("Failed to open project");

    // file_id 格式正确
    assert!(
        file_id.starts_with("file-"),
        "file_id should start with 'file-'"
    );

    // Engine 应该在运行
    assert!(
        state.engine_manager.get_engine(&file_id).is_some(),
        "Engine should be running for opened project"
    );

    // File entry 应该存在
    let files = state.list_open_files();
    assert_eq!(files.len(), 1, "Should have one open file");
    assert_eq!(files[0].0, file_id);
}

#[tokio::test]
async fn test_open_project_existing() {
    let state = AppState::new();
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_path = init_temp_project(tmp_dir.path(), "existing_project").await;
    let path_str = project_path.to_str().unwrap();

    // 第一次打开
    let file_id_1 = project::open_project(path_str, &state)
        .await
        .expect("First open failed");

    // 第二次打开同一路径
    let file_id_2 = project::open_project(path_str, &state)
        .await
        .expect("Second open failed");

    // 应该返回相同的 file_id
    assert_eq!(
        file_id_1, file_id_2,
        "Reopening same project should return same file_id"
    );

    // 只有一个 engine
    let files = state.list_open_files();
    assert_eq!(files.len(), 1, "Should still have just one open file");
}

#[tokio::test]
async fn test_close_project_by_path() {
    let state = AppState::new();
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_path = init_temp_project(tmp_dir.path(), "closeable_project").await;
    let path_str = project_path.to_str().unwrap();

    let file_id = project::open_project(path_str, &state)
        .await
        .expect("Failed to open project");

    // 确认已打开
    assert!(state.engine_manager.get_engine(&file_id).is_some());
    assert_eq!(state.list_open_files().len(), 1);

    // 关闭
    project::close_project(path_str, &state)
        .await
        .expect("Failed to close project");

    // Engine 已关闭
    assert!(
        state.engine_manager.get_engine(&file_id).is_none(),
        "Engine should be shut down after close"
    );

    // File entry 已清理
    assert!(
        state.list_open_files().is_empty(),
        "No files should be open after close"
    );
}

#[tokio::test]
async fn test_close_project_by_id() {
    let state = AppState::new();
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_path = init_temp_project(tmp_dir.path(), "closeable_by_id").await;
    let path_str = project_path.to_str().unwrap();

    let file_id = project::open_project(path_str, &state)
        .await
        .expect("Failed to open project");

    project::close_project_by_id(&file_id, &state)
        .await
        .expect("Failed to close by id");

    assert!(state.engine_manager.get_engine(&file_id).is_none());
    assert!(state.list_open_files().is_empty());
}

#[tokio::test]
async fn test_close_project_not_open() {
    let state = AppState::new();

    let result = project::close_project("/nonexistent/project", &state).await;

    assert!(result.is_err(), "Closing non-open project should fail");
    assert!(result.unwrap_err().contains("is not open"));
}

#[tokio::test]
async fn test_open_multiple_projects() {
    let state = AppState::new();
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let path_a = init_temp_project(tmp_dir.path(), "project_a").await;
    let path_b = init_temp_project(tmp_dir.path(), "project_b").await;

    let id_a = project::open_project(path_a.to_str().unwrap(), &state)
        .await
        .expect("Failed to open project A");

    let id_b = project::open_project(path_b.to_str().unwrap(), &state)
        .await
        .expect("Failed to open project B");

    // 不同项目有不同 file_id
    assert_ne!(id_a, id_b);

    // 两个 engine 都在运行
    assert!(state.engine_manager.get_engine(&id_a).is_some());
    assert!(state.engine_manager.get_engine(&id_b).is_some());

    let files = state.list_open_files();
    assert_eq!(files.len(), 2, "Should have two open files");

    // 关闭一个不影响另一个
    project::close_project_by_id(&id_a, &state)
        .await
        .expect("Failed to close project A");

    assert!(state.engine_manager.get_engine(&id_a).is_none());
    assert!(state.engine_manager.get_engine(&id_b).is_some());
    assert_eq!(state.list_open_files().len(), 1);
}

#[tokio::test]
async fn test_open_project_engine_has_bootstrap_data() {
    let state = AppState::new();
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_path = init_temp_project(tmp_dir.path(), "bootstrapped").await;
    let path_str = project_path.to_str().unwrap();

    let file_id = project::open_project(path_str, &state)
        .await
        .expect("Failed to open project");

    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    // Engine 应该有 bootstrap editor
    let editors = handle.get_all_editors().await;
    assert!(
        !editors.is_empty(),
        "Engine should have at least the bootstrap editor"
    );

    // 应该有 grants
    let grants = handle.get_all_grants().await;
    assert!(!grants.is_empty(), "Engine should have bootstrap grants");
}
