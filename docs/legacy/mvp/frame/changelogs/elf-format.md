# Changelog: L2 elf-format — ZIP 归档 → 目录式项目

> 对应概念文档：`elf-format.md`（Layer 2）
> 分支：`feat/refactor-plan`

---

## 概要

将 `.elf` 文件格式从 ZIP 归档（Phase 1）迁移到 `.elf/` 目录（Phase 2）。核心变更：ElfProject 替代 ElfArchive，SQLite WAL 直接持久化，无需显式 save。采用 Git 模式：system_editor_id 统一从全局配置（`~/.elf/config.json`）读取，项目级 config.toml 不存 editor 信息。

---

## 新增

### `src-tauri/src/elf_project/mod.rs` — ElfProject 主模块

| 方法 | 说明 |
|---|---|
| `ElfProject::init(project_dir)` | 创建 `.elf/` 目录、eventstore.db、config.toml、templates/ |
| `ElfProject::open(project_dir)` | 打开已有 `.elf/` 项目 |
| `event_pool()` | 获取 EventPoolWithPath（Engine 接口不变） |
| `project_dir()` / `elf_dir()` / `db_path()` | 路径访问器 |
| `config()` | 获取 ProjectConfig |

### `src-tauri/src/elf_project/config.rs` — 项目配置（Git 模式）

```toml
[project]
name = "my-project"

[extensions]
enabled = ["document", "task", "session"]
```

- 无 `[editor]` section — system_editor_id 统一从 `~/.elf/config.json` 读取
- `ProjectConfig::new(project_name)` — 创建默认配置（仅需项目名）
- `ProjectConfig::load()` / `save()` — TOML 文件读写

### 新增依赖

| crate | 版本 | 用途 |
|---|---|---|
| `toml` | 0.8 | config.toml 读写 |

---

## 修改

### `src-tauri/src/state.rs` — FileInfo 重构

```rust
// Before (Phase 1)
pub struct FileInfo {
    pub archive: Arc<ElfArchive>,
    pub path: PathBuf,
}

// After (Phase 2)
pub struct FileInfo {
    pub project: Arc<ElfProject>,
}
```

- `get_file_info()` → `get_project()` — 返回 `Arc<ElfProject>`
- `list_open_files()` — 从 project.project_dir() 取路径

### `src-tauri/src/commands/file.rs` — 文件命令重构

| 命令 | 变更 |
|---|---|
| `create_file` | `ElfArchive::new()` → `ElfProject::init()` |
| `open_file` | `ElfArchive::open()` → `ElfProject::open()` |
| `save_file` | **改为 no-op**（SQLite WAL 自动持久化） |
| `close_file` | 不变（Engine shutdown + state cleanup） |
| `rename_file` | 改为更新 config.toml 中的项目名称 |
| `duplicate_file` | 改为返回不支持（目录式项目需文件系统级复制） |
| `get_file_info` | 从 project.config() 取名称，从 db 文件取时间戳 |

- `seed_bootstrap_events()` — system_editor_id 从 GlobalConfig 读取（Git 模式）
- 删除 `bootstrap_elf_meta()` 调用（directory extension 将在 L4 删除）
- 删除 `recover_agent_servers()` 调用（Agent MCP 将在 L4 删除）

### `src-tauri/src/commands/task.rs` — git hooks 路径

```rust
// Before: f.archive.temp_path().join(".elf/git/hooks")
// After:  f.project.elf_dir().join("git/hooks")
```

### `src-tauri/src/commands/editor.rs` — 测试迁移

- 测试环境从 ElfArchive 改为 in-memory EventStore
- system_editor_id fallback 统一从 GlobalConfig 读取

### `src-tauri/src/commands/checkout.rs` — 测试迁移

- 测试环境从 ElfArchive 改为 in-memory EventStore

---

## 删除

### 物理删除 — `src-tauri/src/elf/` 目录

| 文件 | 说明 |
|---|---|
| `src-tauri/src/elf/archive.rs` | ZIP 归档实现（ElfArchive struct，357 行） |
| `src-tauri/src/elf/mod.rs` | 模块声明（`mod archive; pub use archive::ElfArchive;`） |

- `lib.rs` 中 `pub mod elf;` 声明同步删除
- 目录已物理删除，不仅是编译范围移除

### 依赖删除

| crate | 原因 |
|---|---|
| `zip = "0.6"` | ZIP 归档不再需要 |
| `walkdir = "2"` | 递归目录遍历（ZIP 打包用） |
| `whoami = "1"` | 获取用户名（config.toml [editor] 已移除） |

### Config 去重 — `[editor]` section 移除

- `ProjectConfig` 删除 `EditorConfig` struct 和 `[editor]` section
- `ProjectConfig::new()` 签名简化：`(project_name, editor_id, editor_name)` → `(project_name)`
- system_editor_id 统一从 `GlobalConfig`（`~/.elf/config.json`）读取
- 类似 Git：`~/.gitconfig` 存全局身份，`.git/config` 只存项目配置

---

## 集成测试迁移

| 测试文件 | 变更 |
|---|---|
| `tests/engine_block_dir_integration.rs` | ElfArchive → ElfProject |
| `tests/snapshot_integration.rs` | ElfArchive → ElfProject |
| `tests/template_integration.rs` | ElfArchive → ElfProject（仍 #[ignore]） |
| `tests/commands_block_permissions.rs` | ElfArchive → in-memory EventStore |
| `tests/terminal_integration.rs` | ElfArchive → in-memory EventStore（仍 #[ignore]） |

---

## 测试结果

```
471 passed, 0 failed, 28 ignored
```

- 10 个新增 ElfProject 单元测试（mod.rs + config.rs）
- 全部已有测试通过（无回归）
- 28 个 ignored 测试属于待删除模块（terminal、template、directory）

---

## 验证清单

| 概念文档条目 | 实现状态 |
|---|---|
| `.elf/` 目录结构（eventstore.db + config.toml + templates/） | ✅ |
| config.toml 两段结构（project + extensions，无 editor） | ✅ |
| config.toml 只存静态项目配置（Git 模式） | ✅ |
| system_editor_id 统一从 GlobalConfig 读取 | ✅ |
| elf init 流程（创建目录 + bootstrap events） | ✅ |
| ZIP 归档完全移除（物理删除 + 依赖删除） | ✅ |
| save_file 是 no-op（SQLite WAL） | ✅ |
| Bootstrap events 直接写入 EventStore | ✅ |
| 无向后兼容（干净迁移） | ✅ |
