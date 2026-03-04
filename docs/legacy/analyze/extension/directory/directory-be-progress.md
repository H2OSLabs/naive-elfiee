# Directory Extension 后端开发进度清单

**版本**: v1.0
**分支**: feat/directory-extension-redesign
**参考文档**: directory-final.md

---

## Phase 0: 准备工作

### 0.1 环境检查

- [ ] 确认当前在 `dev` 分支
  ```bash
  git branch --show-current  # 应该显示 dev
  ```

- [ ] 确认工作区干净
  ```bash
  git status  # 应该无未提交更改
  ```

- [ ] 安装 elfiee-ext-gen
  ```bash
  cd elfiee-ext-gen
  cargo install --path . --force
  elfiee-ext-gen --version  # 验证安装成功
  cd ..
  ```

### 0.2 备份旧实现

- [ ] 创建备份分支
  ```bash
  git checkout feat/extension-directory
  git checkout -b backup/directory-old-design
  git push origin backup/directory-old-design
  ```

- [ ] 验证备份
  ```bash
  git log --oneline -5  # 确认最新提交
  ```

### 0.3 创建开发分支

- [ ] 从 dev 创建新分支
  ```bash
  git checkout dev
  git pull origin dev
  git checkout -b feat/directory-extension-redesign
  ```

- [ ] 验证分支
  ```bash
  git branch --show-current  # 应该显示 feat/directory-extension-redesign
  ```

### 0.4 删除旧实现

- [ ] 删除旧 directory extension
  ```bash
  rm -rf src-tauri/src/extensions/directory/
  ```

- [ ] 验证删除
  ```bash
  ls src-tauri/src/extensions/  # 应该没有 directory/
  ```

- [ ] 提交删除
  ```bash
  git add -A
  git commit -m "chore: remove old directory extension for redesign"
  ```

---

## Phase 1: 使用 Generator 生成骨架

### 1.1 生成扩展骨架

- [ ] 运行 generator（在项目根目录）
  ```bash
  elfiee-ext-gen create \
    --name directory \
    --block-type directory \
    --capabilities root,scan,list,exportall,refresh,create,delete,rename,search \
    --with-auth-tests \
    --with-workflow-tests
  ```

- [ ] 验证生成的文件
  ```bash
  ls -la src-tauri/src/extensions/directory/
  # 应该看到:
  # - mod.rs
  # - directory_root.rs
  # - directory_scan.rs
  # - directory_list.rs
  # - directory_exportall.rs
  # - directory_refresh.rs
  # - directory_create.rs
  # - directory_delete.rs
  # - directory_rename.rs
  # - directory_search.rs
  # - tests.rs
  # - DEVELOPMENT_GUIDE.md
  ```

- [ ] 检查生成的测试框架
  ```bash
  cd src-tauri
  cargo test directory::tests 2>&1 | grep "test result"
  # 预期：大部分测试失败（因为都是 todo!()）
  cd ..
  ```

### 1.2 初次运行 guide

- [ ] 生成开发指南
  ```bash
  elfiee-ext-gen guide directory
  ```

- [ ] 查看输出，了解需要实现的内容
  - 记录失败的测试数量
  - 记录 Payload 定义的 TODO 数量
  - 记录 Handler 实现的 TODO 数量

- [ ] 提交生成的骨架
  ```bash
  git add -A
  git commit -m "feat: generate directory extension skeleton with elfiee-ext-gen"
  ```

---

## Phase 2: 定义 Payload 结构

### 2.1 复制可复用代码

- [ ] 从备份分支复制 utils.rs
  ```bash
  git checkout backup/directory-old-design -- src-tauri/src/extensions/directory/utils.rs
  ```

- [ ] 检查 utils.rs 内容
  ```bash
  grep -n "pub(super) fn" src-tauri/src/extensions/directory/utils.rs
  # 应该看到: read_dir_single, read_dir_recursive
  ```

- [ ] 提交复用代码
  ```bash
  git add src-tauri/src/extensions/directory/utils.rs
  git commit -m "feat: reuse utils.rs from old implementation"
  ```

### 2.2 定义 FileEntry 结构

- [ ] 编辑 `src-tauri/src/extensions/directory/mod.rs`
- [ ] 在 Payload 定义之前添加 FileEntry
  ```rust
  /// 文件映射条目
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct FileEntry {
      /// 对应的 Block ID
      pub block_id: String,

      /// 文件最后修改时间（RFC3339 格式）
      pub last_modified: String,
  }
  ```

- [ ] 保存文件

### 2.3 定义 DirectoryRootPayload

**参考**: directory-final.md § 4.2

- [ ] 编辑 `mod.rs`，找到 `DirectoryRootPayload`
- [ ] 替换为以下内容：
  ```rust
  /// 挂载项目目录到 Directory Block
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryRootPayload {
      /// 项目根目录绝对路径
      pub root: String,

      /// 默认是否递归
      #[serde(default = "default_true")]
      pub recursive: bool,

      /// 默认是否包含隐藏文件
      #[serde(default)]
      pub include_hidden: bool,

      /// 默认最大深度
      #[serde(default)]
      pub max_depth: Option<usize>,
  }
  ```

- [ ] 添加 default_true 函数
  ```rust
  fn default_true() -> bool { true }
  ```

### 2.4 定义 DirectoryScanPayload

**参考**: directory-final.md § 4.3

- [ ] 替换 `DirectoryScanPayload`：
  ```rust
  /// 批量扫描文件，导入为 Blocks
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryScanPayload {
      /// 是否递归扫描（覆盖 contents.recursive）
      #[serde(default)]
      pub recursive: Option<bool>,

      /// 是否包含隐藏文件（覆盖 contents.include_hidden）
      #[serde(default)]
      pub include_hidden: Option<bool>,

      /// 最大递归深度（覆盖 contents.max_depth）
      #[serde(default)]
      pub max_depth: Option<usize>,
  }
  ```

### 2.5 定义 DirectoryListPayload

**参考**: directory-final.md § 4.4

- [ ] 替换 `DirectoryListPayload`：
  ```rust
  /// 列出 indexed_files
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryListPayload {
      /// 文件名模式（可选）
      #[serde(default)]
      pub pattern: Option<String>,
  }
  ```

### 2.6 定义 DirectoryExportallPayload

**参考**: directory-final.md § 4.5

- [ ] 替换 `DirectoryExportallPayload`：
  ```rust
  /// 批量导出 Blocks 到文件
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryExportallPayload {
      /// 要导出的项目列表
      pub exports: Vec<ExportItem>,
  }

  /// 单个导出项
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct ExportItem {
      /// Block ID
      pub block_id: String,

      /// 内容（前端提供）
      pub content: String,
  }
  ```

### 2.7 定义 DirectoryRefreshPayload

**参考**: directory-final.md § 4.6

- [ ] 替换 `DirectoryRefreshPayload`：
  ```rust
  /// 检测外部文件变更并同步
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryRefreshPayload {
      /// 可选：指定文件路径（默认全部刷新）
      #[serde(default)]
      pub file_path: Option<String>,
  }
  ```

### 2.8 定义 DirectoryCreatePayload

**参考**: directory-final.md § 4.7

- [ ] 替换 `DirectoryCreatePayload`：
  ```rust
  /// 创建文件并创建对应 Block
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryCreatePayload {
      /// 文件路径（相对于 contents.root）
      pub path: String,

      /// 类型："file" | "dir"
      pub item_type: String,

      /// 文件内容（可选，默认空）
      #[serde(default)]
      pub content: String,
  }
  ```

### 2.9 定义 DirectoryDeletePayload

**参考**: directory-final.md § 4.8

- [ ] 替换 `DirectoryDeletePayload`：
  ```rust
  /// 删除文件并软删除对应 Block
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryDeletePayload {
      /// 文件路径（相对于 contents.root）
      pub path: String,

      /// 是否递归删除目录
      #[serde(default)]
      pub recursive: bool,
  }
  ```

### 2.10 定义 DirectoryRenamePayload

**参考**: directory-final.md § 4.9

- [ ] 替换 `DirectoryRenamePayload`：
  ```rust
  /// 重命名文件并更新对应 Block
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectoryRenamePayload {
      /// 旧路径（相对于 contents.root）
      pub old_path: String,

      /// 新路径（相对于 contents.root）
      pub new_path: String,
  }
  ```

### 2.11 定义 DirectorySearchPayload

**参考**: directory-final.md § 4.10

- [ ] 替换 `DirectorySearchPayload`：
  ```rust
  /// 搜索文件名或内容
  #[derive(Debug, Clone, Serialize, Deserialize, Type)]
  pub struct DirectorySearchPayload {
      /// 搜索模式（支持通配符）
      pub pattern: String,

      /// 是否递归搜索
      #[serde(default = "default_true")]
      pub recursive: bool,

      /// 是否包含隐藏文件
      #[serde(default)]
      pub include_hidden: bool,
  }
  ```

### 2.12 验证 Payload 编译

- [ ] 编译检查
  ```bash
  cd src-tauri
  cargo check --package elfiee
  # 应该无编译错误
  cd ..
  ```

- [ ] 提交 Payload 定义
  ```bash
  git add src-tauri/src/extensions/directory/mod.rs
  git commit -m "feat: define all directory extension payloads"
  ```

- [ ] 运行 Payload 测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_.*_payload -- --nocapture
  # 预期：所有 Payload 反序列化测试通过
  cd ..
  ```

---

## Phase 3-11: TDD 循环实现（每个 Capability 一个迭代）

### TDD 循环说明

每个 capability 遵循严格的 TDD 流程：

1. **Red（失败）**: 运行测试，确认失败（handler 未实现）
2. **Green（通过）**: 实现最小代码使测试通过
3. **Refactor（重构）**: 优化代码（可选）
4. **Guide**: 使用 `elfiee-ext-gen guide` 验证进度
5. **Commit**: 提交该 capability 的实现

---

## Phase 3: directory.root ✅

### 3.1 RED - 运行测试确认失败

- [x] 运行 directory.root 测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_root -- --nocapture
  cd ..
  ```

- [x] 预期结果：所有 4 个测试失败
  - `test_root_basic` - FAIL (todo!())
  - `test_root_authorization_owner` - FAIL (todo!())
  - `test_root_authorization_non_owner_without_grant` - FAIL (todo!())
  - `test_root_authorization_non_owner_with_grant` - FAIL (todo!())

### 3.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.2

- [x] 编辑 `src-tauri/src/extensions/directory/directory_root.rs`
- [x] 添加必要的导入
  ```rust
  use std::path::Path;
  use super::DirectoryRootPayload;
  use crate::capabilities::core::create_event;
  ```

- [x] 实现 `handle_root` 函数（参考 directory-final.md § 4.2 完整代码）

- [x] 关键实现要点：
  - [x] 反序列化 payload
  - [x] 验证 root 路径存在且是目录
  - [x] 使用 `canonicalize()` 规范化路径
  - [x] 生成 Event：
    - entity: `block.block_id`
    - attribute: "directory.root"
    - value.contents: 包含 root, recursive, include_hidden, max_depth, indexed_files, last_updated, watch_enabled

- [x] 编译验证
  ```bash
  cd src-tauri
  cargo check --package elfiee
  cd ..
  ```

### 3.3 GREEN - 验证测试通过

- [x] 再次运行 directory.root 测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_root -- --nocapture
  cd ..
  ```

- [x] 预期结果：所有 5 个测试通过 ✅
  - `test_root_basic` - ok
  - `test_root_payload_deserialize` - ok
  - `test_root_authorization_owner` - ok
  - `test_root_authorization_non_owner_without_grant` - ok
  - `test_root_authorization_non_owner_with_grant` - ok

- [x] 修复测试路径问题（在 test_root_basic 中创建临时目录）

### 3.4 GUIDE - 验证进度

- [x] 运行 guide 命令
  ```bash
  elfiee-ext-gen guide directory
  ```

- [x] 检查输出：
  - [x] directory.root 应该标记为"实现完成"
  - [x] 测试通过率应该提升（至少 5/46 通过）

### 3.5 COMMIT - 提交实现

- [x] 提交代码
  ```bash
  git add src-tauri/src/extensions/directory/directory_root.rs
  git add src-tauri/src/extensions/directory/tests.rs
  git commit -m "feat: implement directory.root handler

- Validate and canonicalize root path
- Store directory configuration in Block.contents
- Fix test by creating temporary test directory
- Pass all 5 directory.root tests (including payload test)"
  ```

---

## Phase 4: directory.scan ✅

### 4.1 RED - 运行测试确认失败

- [x] 运行 directory.scan 测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_scan -- --nocapture
  cd ..
  ```

- [x] 预期结果：所有 4 个测试失败 (todo!())

### 4.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.3

- [x] 编辑 `src-tauri/src/extensions/directory/directory_scan.rs`
- [x] 添加依赖导入
  ```rust
  use std::collections::HashMap;
  use std::fs;
  use std::path::Path;
  use walkdir::WalkDir;
  use super::{DirectoryScanPayload, FileEntry};
  use crate::capabilities::core::create_event;
  ```

- [x] 实现 `handle_scan` 函数（参考 directory-final.md § 4.3）

- [x] 关键实现要点：
  - [x] 从 `block.contents.root` 读取项目根目录
  - [x] 如果无 root，返回错误："Directory block has no root. Call directory.root first."
  - [x] 读取默认配置（recursive, include_hidden, max_depth）
  - [x] payload 可覆盖默认配置
  - [x] 遍历文件系统（使用 walkdir）
  - [x] 为每个文件生成 `core.create` Event（block_type: "markdown"）
  - [x] 记录 indexed_files: `HashMap<String, FileEntry>`
  - [x] 生成 `directory.scan` Event 更新 Block.contents

- [x] 修复类型错误：使用 `.ok()` 转换不同 Result 类型
- [x] 修复测试：添加临时目录和 block.contents 初始化
- [x] 添加测试清理代码

- [x] 编译验证
  ```bash
  cd src-tauri
  cargo check --package elfiee
  cd ..
  ```

### 4.3 GREEN - 验证测试通过

- [x] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_scan -- --nocapture
  cd ..
  ```

- [x] 预期：所有 5 个测试通过 ✅

### 4.4 GUIDE - 验证进度

- [x] 运行 guide 命令
  ```bash
  elfiee-ext-gen guide directory
  ```

- [x] 检查：directory.scan 标记为完成

### 4.5 COMMIT - 提交实现

- [x] 提交
  ```bash
  git add .
  git commit -m "feat(directory): implement directory.scan capability

- Add directory_scan.rs handler with file traversal
- Support recursive/non-recursive scanning
- Support include_hidden and max_depth options
- Generate core.create events for each file
- Update block.contents with indexed_files mapping
- All 5 tests passing"
  ```

---

## Phase 5: directory.list ✅

### 5.1 RED - 运行测试确认失败

- [x] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_list -- --nocapture
  cd ..
  ```

### 5.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.4

- [x] 编辑 `src-tauri/src/extensions/directory/directory_list.rs`
- [x] 实现 `handle_list` 函数
- [x] 关键点：
  - [x] 从 `block.contents.indexed_files` 读取文件映射
  - [x] 实现简单通配符过滤（`*.ext`, `prefix*`, 子串匹配）
  - [x] 生成 Read Event（entity 是 editor_id）

### 5.2.1 重构 - 移除冗余的 directory.search

- [x] 删除 `directory_search.rs` 文件
- [x] 从 `mod.rs` 移除 DirectorySearchPayload 和模块引用
- [x] 从 `tests.rs` 删除 5 个 search 测试
- [x] 从 `registry.rs` 移除 DirectorySearchCapability 注册
- [x] 从 `lib.rs` 移除 DirectorySearchPayload 类型导出
- [x] 更新 progress 文档：Phase 11 → 最终验证，测试总数 46 → 41

**原因**: directory.search 功能与 directory.list + directory.refresh 重复

### 5.3 GREEN - 验证测试通过

- [x] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_list -- --nocapture
  cd ..
  ```

- [x] 预期：所有 5 个测试通过 ✅

### 5.4 GUIDE - 验证进度

- [x] 运行 guide
  ```bash
  elfiee-ext-gen guide directory
  ```

### 5.5 COMMIT - 提交实现

- [x] 提交
  ```bash
  git add .
  git commit -m "feat(directory): implement directory.list and remove redundant directory.search"
  ```

---

## Phase 6: directory.exportall ✅

### 6.1 RED - 运行测试确认失败

- [x] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_exportall -- --nocapture
  cd ..
  ```

### 6.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.5

- [x] 编辑 `src-tauri/src/extensions/directory/directory_exportall.rs`
- [x] 实现 `handle_exportall` 函数
- [x] 关键点：
  - [x] 从 contents.root 读取项目根目录
  - [x] 从 contents.indexed_files 读取映射
  - [x] **静默跳过**不在 indexed_files 中的 Block（Elfiee 内部笔记等）
  - [x] 遍历 payload.exports：查找路径 → 写入文件 → 更新 last_modified
  - [x] 生成 N 个 `directory.export-block` Events
  - [x] 生成 1 个 `directory.update-index` Event

**设计改进**: 只导出 indexed_files 中的 Block，跳过非文件 Block，避免前端需要预先过滤

### 6.3 GREEN - 验证测试通过

- [x] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_exportall -- --nocapture
  cd ..
  ```

- [x] 预期：所有 5 个测试通过 ✅

### 6.4 GUIDE - 验证进度

- [x] 运行 guide
  ```bash
  elfiee-ext-gen guide directory
  ```

### 6.5 COMMIT - 提交实现

- [x] 提交
  ```bash
  git add .
  git commit -m "feat(directory): implement directory.exportall capability"
  ```

---

## Phase 7: directory.refresh

### 7.1 RED - 运行测试确认失败

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_refresh -- --nocapture
  cd ..
  ```

### 7.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.6

- [ ] 编辑 `src-tauri/src/extensions/directory/directory_refresh.rs`
- [ ] 实现 `handle_refresh` 函数
- [ ] 关键点：
  - [ ] 支持 file_path 指定单文件刷新
  - [ ] 读取文件 mtime，与 entry.last_modified 对比
  - [ ] 如果不同，生成 `markdown.write` Event
  - [ ] 更新 indexed_files 中的 last_modified
  - [ ] 生成 `directory.refresh` Event（包含 detected_changes）

### 7.3 GREEN - 验证测试通过

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_refresh -- --nocapture
  cd ..
  ```

- [ ] 预期：所有 4 个测试通过 ✅

### 7.4 GUIDE - 验证进度

- [ ] 运行 guide
  ```bash
  elfiee-ext-gen guide directory
  ```

### 7.5 COMMIT - 提交实现

- [ ] 提交
  ```bash
  git add src-tauri/src/extensions/directory/directory_refresh.rs
  git commit -m "feat: implement directory.refresh handler

- Detect external file changes
- Sync modifications to Elfiee Blocks
- Pass all 4 directory.refresh tests"
  ```

---

## Phase 8: directory.create

### 8.1 RED - 运行测试确认失败

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_create -- --nocapture
  cd ..
  ```

### 8.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.7

- [ ] 编辑 `src-tauri/src/extensions/directory/directory_create.rs`
- [ ] 实现 `handle_create` 函数
- [ ] 关键点：
  - [ ] 支持 item_type: "file" | "dir"
  - [ ] 如果是 dir：创建目录，仅记录操作，不创建 Block
  - [ ] 如果是 file：创建文件 + 生成 `core.create` Event
  - [ ] 更新 indexed_files
  - [ ] 生成 `directory.create` Event

### 8.3 GREEN - 验证测试通过

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_create -- --nocapture
  cd ..
  ```

- [ ] 预期：所有 4 个测试通过 ✅

### 8.4 GUIDE - 验证进度

- [ ] 运行 guide
  ```bash
  elfiee-ext-gen guide directory
  ```

### 8.5 COMMIT - 提交实现

- [ ] 提交
  ```bash
  git add src-tauri/src/extensions/directory/directory_create.rs
  git commit -m "feat: implement directory.create handler

- Create files/directories in project
- Auto-create corresponding Blocks
- Pass all 4 directory.create tests"
  ```

---

## Phase 9: directory.delete

### 9.1 RED - 运行测试确认失败

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_delete -- --nocapture
  cd ..
  ```

### 9.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.8

- [ ] 编辑 `src-tauri/src/extensions/directory/directory_delete.rs`
- [ ] 实现 `handle_delete` 函数
- [ ] 关键点：
  - [ ] 从 indexed_files 获取 block_id
  - [ ] 删除外部文件（支持 recursive）
  - [ ] 生成 `core.delete` Event（软删除）
  - [ ] 从 indexed_files 移除
  - [ ] 生成 `directory.delete` Event

### 9.3 GREEN - 验证测试通过

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_delete -- --nocapture
  cd ..
  ```

- [ ] 预期：所有 4 个测试通过 ✅

### 9.4 GUIDE - 验证进度

- [ ] 运行 guide
  ```bash
  elfiee-ext-gen guide directory
  ```

### 9.5 COMMIT - 提交实现

- [ ] 提交
  ```bash
  git add src-tauri/src/extensions/directory/directory_delete.rs
  git commit -m "feat: implement directory.delete handler

- Delete files and soft-delete Blocks
- Support recursive directory deletion
- Pass all 4 directory.delete tests"
  ```

---

## Phase 10: directory.rename

### 10.1 RED - 运行测试确认失败

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_rename -- --nocapture
  cd ..
  ```

### 10.2 GREEN - 实现 Handler

**参考**: directory-final.md § 4.9

- [ ] 编辑 `src-tauri/src/extensions/directory/directory_rename.rs`
- [ ] 实现 `handle_rename` 函数
- [ ] 关键点：
  - [ ] 重命名外部文件
  - [ ] 从 indexed_files 移除旧路径，添加新路径
  - [ ] 更新 last_modified
  - [ ] 生成 `core.update-name` Event
  - [ ] 生成 `directory.rename` Event

### 10.3 GREEN - 验证测试通过

- [ ] 运行测试
  ```bash
  cd src-tauri
  cargo test directory::tests::test_rename -- --nocapture
  cd ..
  ```

- [ ] 预期：所有 4 个测试通过 ✅

### 10.4 GUIDE - 验证进度

- [ ] 运行 guide
  ```bash
  elfiee-ext-gen guide directory
  ```

### 10.5 COMMIT - 提交实现

- [ ] 提交
  ```bash
  git add src-tauri/src/extensions/directory/directory_rename.rs
  git commit -m "feat: implement directory.rename handler

- Rename files and update Block names
- Maintain indexed_files consistency
- Pass all 4 directory.rename tests"
  ```

---

## Phase 11: 最终验证

### 11.1 运行完整测试套件

- [ ] 运行所有 directory 测试
  ```bash
  cd src-tauri
  cargo test directory::tests -- --nocapture 2>&1 | tee /tmp/directory_tests.log
  cd ..
  ```

- [ ] 统计测试结果
  ```bash
  grep "test result:" /tmp/directory_tests.log
  ```

- [ ] 预期结果：
  - Payload Tests: 8/8 通过
  - Functionality Tests: 8/8 通过
  - Authorization Tests: 24/24 通过（8 capabilities × 3 tests）
  - Workflow Tests: 1/1 通过
  - **Total: 41/41 tests passed** ✅

### 11.2 运行 Guide 检查

- [ ] 运行 guide 命令
  ```bash
  elfiee-ext-gen guide directory
  ```

- [ ] 检查输出：
  - [ ] 所有 capabilities 标记为"实现完成"
  - [ ] 所有 Payload 定义完成
  - [ ] 所有测试通过

### 11.3 代码质量检查

- [ ] 运行 clippy
  ```bash
  cd src-tauri
  cargo clippy --package elfiee -- -D warnings
  cd ..
  ```

- [ ] 运行 fmt 检查
  ```bash
  cd src-tauri
  cargo fmt --check
  cd ..
  ```

- [ ] 如有问题，修复并重新验证

### 11.4 文档更新

- [ ] 检查 DEVELOPMENT_GUIDE.md
  - [ ] 所有 TODO 已移除
  - [ ] Payload 示例已更新
  - [ ] Handler 实现说明完整

- [ ] 提交文档更新（如有修改）
  ```bash
  git add src-tauri/src/extensions/directory/DEVELOPMENT_GUIDE.md
  git commit -m "docs: update directory extension development guide"
  ```

### 11.5 最终提交

- [ ] 创建总结提交（可选）
  ```bash
  git commit --allow-empty -m "feat: complete directory extension implementation

All 9 capabilities implemented and tested:
- directory.root: Mount project directory
- directory.scan: Batch import files
- directory.list: List indexed files
- directory.exportall: Export Blocks to filesystem
- directory.refresh: Sync external changes
- directory.create: Create files/directories
- directory.delete: Delete files
- directory.rename: Rename files

Test Results: 41/41 passed ✅

Note: directory.search removed as redundant (use directory.list + directory.refresh instead)"
  ```

---

## Phase 12: 集成验证（可选）

### 12.1 构建完整项目

- [ ] 构建 Tauri 应用
  ```bash
  pnpm tauri build --debug
  ```

- [ ] 检查构建输出，确认无错误

### 13.2 手动测试（如有前端）

- [ ] 启动开发服务器
  ```bash
  pnpm tauri dev
  ```

- [ ] 测试核心工作流：
  - [ ] 创建 .elf 文件
  - [ ] 创建 directory Block
  - [ ] 调用 directory.root 挂载目录
  - [ ] 调用 directory.scan 导入文件
  - [ ] 编辑 Block
  - [ ] 调用 directory.exportall 导出
  - [ ] 外部修改文件
  - [ ] 调用 directory.refresh 同步

### 13.3 合并到 dev 分支

- [ ] 确认所有测试通过
  ```bash
  cd src-tauri
  cargo test
  cd ..
  ```

- [ ] 推送分支
  ```bash
  git push origin feat/directory-extension-redesign
  ```

- [ ] 创建 Pull Request 到 dev
  - Title: `feat: redesign and implement directory extension`
  - Description: 列出所有实现的 capabilities 和测试结果

- [ ] Code Review 后合并

---

## 附录：故障排查

### 测试失败的常见原因

1. **Payload 反序列化失败**
   - 检查 tests.rs 中的 JSON 是否匹配 Payload 定义
   - 确认必填字段都有提供

2. **Handler panic**
   - 检查 `block.contents.get("field")` 的空值处理
   - 确认文件路径操作的错误处理

3. **授权测试失败**
   - 检查 capability 宏的 `target` 是否正确
   - 确认 grants 表正确设置

4. **Event 结构错误**
   - 检查 Event.entity 是否正确（Block ID 或 Editor ID）
   - 确认 Event.attribute 格式 `"{editor_id}/{cap_id}"`

### 回滚策略

如果某个 capability 实现遇到严重问题：

```bash
# 回滚到上一个工作的 commit
git log --oneline -10  # 查看历史
git reset --hard <commit-hash>

# 或者只回滚特定文件
git checkout HEAD~1 -- src-tauri/src/extensions/directory/directory_XXX.rs
```

---

## 完成标志

当以下所有条件满足时，Directory Extension 开发完成：

- [x] Phase 0: 准备工作完成
- [x] Phase 1: 扩展骨架生成
- [x] Phase 2: Payload 定义完成
- [ ] Phase 3-11: 所有 9 个 capabilities 实现并测试通过
- [ ] Phase 12: 最终验证通过（46/46 tests）
- [ ] Phase 13: 集成验证完成（可选）

**最终目标**: 100% 测试通过率，代码无 clippy warnings，准备合并到 dev 分支。
