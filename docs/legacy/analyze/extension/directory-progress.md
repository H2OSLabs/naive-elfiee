# Directory Extension 开发进度

本文档记录 directory extension 的开发进度，按照 TDD 流程组织。

**更新日期**：2025-11-06

---

## 总体进度

- [x] 所有 Payload 定义完成（7/7）
- [x] 所有 Handler 实现完成（7/7）
- [x] 所有测试通过（36/36）
- [x] 验证通过
- [ ] Tauri 集成测试通过（待测试）

---

## Phase 1: 初始化

### 1.1 生成扩展骨架

```bash
cd /home/yaosh/projects/elfiee
elfiee-ext-gen create -n directory -b directory -c list,create,delete,rename,refresh,watch,search
```

- [x] 生成成功
- [x] 文件结构正确（10个文件）
- [x] 注册文件更新（mod.rs, registry.rs, lib.rs）

### 1.2 初始测试基线

```bash
cd src-tauri
cargo test directory::tests --lib
```

**预期**：21 passed (授权测试), 15 failed (payload + 功能 + 工作流)

- [x] 测试运行成功
- [x] 授权测试全部通过（21/21）
- [x] 其他测试符合预期失败

---

## Phase 2: Payload 定义（批量完成）

编辑 `src-tauri/src/extensions/directory/mod.rs`

### 2.1 定义所有 Payload 结构体

- [x] `DirectoryListPayload`（root, recursive, include_hidden, max_depth）
- [x] `DirectoryCreatePayload`（path, item_type, content）
- [x] `DirectoryDeletePayload`（path, recursive）
- [x] `DirectoryRenamePayload`（old_path, new_path）
- [x] `DirectoryRefreshPayload`（recursive）
- [x] `DirectoryWatchPayload`（enabled）
- [x] `DirectorySearchPayload`（pattern, recursive, include_hidden）

### 2.2 更新 Payload 测试

编辑 `src-tauri/src/extensions/directory/tests.rs`，为每个 Payload 测试提供正确的 JSON 示例

- [x] `test_list_payload_deserialize`
- [x] `test_create_payload_deserialize`
- [x] `test_delete_payload_deserialize`
- [x] `test_rename_payload_deserialize`
- [x] `test_refresh_payload_deserialize`
- [x] `test_watch_payload_deserialize`
- [x] `test_search_payload_deserialize`

### 2.3 验证 Payload 测试

```bash
cargo test directory::tests::test_list_payload_deserialize
# 重复上述命令，测试所有 payload
```

**预期**：28 passed (21 授权 + 7 payload), 8 failed (7 功能 + 1 工作流)

- [x] 所有 Payload 测试通过（7/7）

---

## Phase 3: 能力实现（逐个完成）

### 3.1 directory.list

**文件**：`src-tauri/src/extensions/directory/directory_list.rs`

#### 实现步骤
- [x] 实现 handler 逻辑（递归列表、隐藏文件、深度限制）
- [x] 路径安全检查（canonicalize + root boundary）
- [x] 时间戳使用 `chrono::Utc::now().to_rfc3339()`

#### 测试
```bash
cargo test directory::tests::test_list_basic
```

- [x] 功能测试通过
- [x] 边界条件测试补充（hidden files, max_depth）

#### 验证
```bash
cd /home/yaosh/projects/elfiee
elfiee-ext-gen guide directory
```

- [x] Guide 显示 list 相关测试通过

---

### 3.2 directory.create

**文件**：`src-tauri/src/extensions/directory/directory_create.rs`

#### 实现步骤
- [x] 反序列化 payload
- [x] 验证 `item_type`（"file" 或 "dir"）
- [x] 路径安全检查
- [x] 检查路径不存在
- [x] 创建文件（默认空）或目录
- [x] 生成事件

#### 测试
```bash
cargo test directory::tests::test_create_basic
```

- [x] 功能测试通过
- [x] 边界条件测试补充（duplicate file, empty content）

#### 验证
```bash
elfiee-ext-gen guide directory
```

- [x] Guide 显示 create 相关测试通过

---

### 3.3 directory.delete

**文件**：`src-tauri/src/extensions/directory/directory_delete.rs`

#### 实现步骤
- [x] 验证路径存在且在 root 范围内
- [x] 检查目录删除需要 `recursive=true`
- [x] 执行删除（`fs::remove_file` 或 `fs::remove_dir_all`）
- [x] 生成事件

#### 测试
```bash
cargo test directory::tests::test_delete_basic
```

- [x] 功能测试通过
- [x] 边界条件测试补充（non-empty dir without recursive）

#### 验证
```bash
elfiee-ext-gen guide directory
```

- [x] Guide 显示 delete 相关测试通过

---

### 3.4 directory.rename

**文件**：`src-tauri/src/extensions/directory/directory_rename.rs`

#### 实现步骤
- [x] 验证 `old_path` 存在
- [x] 验证 `new_path` 不存在
- [x] 验证两个路径都在 root 范围内
- [x] 执行重命名（`fs::rename`）
- [x] 生成事件

#### 测试
```bash
cargo test directory::tests::test_rename_basic
```

- [x] 功能测试通过
- [x] 边界条件测试补充（rename to existing name）

#### 验证
```bash
elfiee-ext-gen guide directory
```

- [x] Guide 显示 rename 相关测试通过

---

### 3.5 directory.refresh

**文件**：`src-tauri/src/extensions/directory/directory_refresh.rs`

#### 实现步骤
- [x] 复用 `directory.list` 的逻辑
- [x] 根据 `recursive` 参数重新扫描
- [x] 更新 `Block.contents.entries` 和 `last_updated`
- [x] 生成事件

#### 测试
```bash
cargo test directory::tests::test_refresh_basic
```

- [x] 功能测试通过
- [x] 工作流测试补充（external change + refresh）

#### 验证
```bash
elfiee-ext-gen guide directory
```

- [x] Guide 显示 refresh 相关测试通过

---

### 3.6 directory.watch

**文件**：`src-tauri/src/extensions/directory/directory_watch.rs`

#### 实现步骤
- [x] 设置 `Block.contents.watch_enabled` 标志
- [x] 生成事件（仅标志变更，不实现真正的监听）

#### 测试
```bash
cargo test directory::tests::test_watch_basic
```

- [x] 功能测试通过

#### 验证
```bash
elfiee-ext-gen guide directory
```

- [x] Guide 显示 watch 相关测试通过

---

### 3.7 directory.search

**文件**：`src-tauri/src/extensions/directory/directory_search.rs`

#### 实现步骤
- [x] 反序列化 payload
- [x] 验证 `pattern` 不为空
- [x] 遍历文件系统（支持 recursive）
- [x] 文件名模式匹配（支持通配符 `*`、`?`）
- [x] 过滤隐藏文件
- [x] 生成事件（包含匹配的文件列表）

#### 测试
```bash
cargo test directory::tests::test_search_basic
```

- [x] 功能测试通过
- [x] 工作流测试补充（search with pattern）

#### 验证
```bash
elfiee-ext-gen guide directory
```

- [x] Guide 显示 search 相关测试通过

---

## Phase 4: 工作流测试

### 4.1 完善 test_full_workflow

编辑 `src-tauri/src/extensions/directory/tests.rs`

#### 测试场景
- [x] list 空目录
- [x] create 创建文件
- [x] list 验证文件出现
- [x] rename 重命名文件
- [x] search 搜索文件
- [x] refresh 刷新缓存
- [x] watch 启用监听
- [x] delete 删除文件
- [x] list 验证文件消失

#### 运行测试
```bash
cargo test directory::tests::test_full_workflow
```

- [x] 工作流测试通过

---

## Phase 5: 完整验证

### 5.1 运行所有测试

```bash
cd /home/yaosh/projects/elfiee/src-tauri
cargo test directory::tests --lib
```

**预期**：36 passed (7 payload + 7 功能 + 21 授权 + 1 工作流)

- [x] 所有测试通过（36/36）
- [x] 无编译警告

### 5.2 Validate 验证

```bash
cd /home/yaosh/projects/elfiee
elfiee-ext-gen validate directory
```

- [x] 模块导出正确
- [x] Capability 已注册（7个）
- [x] Specta 类型已注册（7个）

### 5.3 Guide 最终检查

```bash
elfiee-ext-gen guide directory
```

**预期**：100% 测试通过，无失败项

- [x] Guide 显示所有测试通过
- [x] 无 TODO 或失败提示

---

## Phase 6: Tauri 集成测试

### 6.1 启动应用

```bash
cd /home/yaosh/projects/elfiee
pnpm tauri dev
```

- [ ] 应用启动成功
- [ ] 无编译错误

### 6.2 前端测试

在浏览器控制台测试所有能力：

```typescript
// 测试 directory.list
await invoke('execute_command', {
  cmd: {
    cmd_id: crypto.randomUUID(),
    editor_id: 'user1',
    cap_id: 'directory.list',
    block_id: 'test-block',
    payload: { recursive: true, include_hidden: false },
  }
});

// 测试 directory.create
// ... 重复测试所有 7 个能力
```

- [ ] `directory.list` 正常工作
- [ ] `directory.create` 正常工作
- [ ] `directory.delete` 正常工作
- [ ] `directory.rename` 正常工作
- [ ] `directory.refresh` 正常工作
- [ ] `directory.watch` 正常工作
- [ ] `directory.search` 正常工作

---

## 完成标志

- [x] Phase 1-5 完成（后端实现与测试）
- [ ] Phase 6 完成（Tauri 集成测试）
- [ ] 代码已提交
- [x] 文档已更新
- [ ] 可以演示完整功能

---

**最后更新**：2025-11-06
**状态**：后端开发完成，待 Tauri 集成测试

## 开发总结

### 已完成
- ✅ 所有 7 个 Payload 定义
- ✅ 所有 7 个 Handler 实现
- ✅ 36 个测试全部通过（无警告）
- ✅ Validate 验证通过
- ✅ Guide 100% 完成

### 技术亮点
1. **路径安全**：使用 `canonicalize()` 防止路径遍历攻击
2. **递归支持**：list/search 支持递归和深度限制
3. **模式匹配**：search 实现通配符 `*` 和 `?` 支持
4. **职责分离**：rename 只操作文件系统，refresh 负责缓存同步
5. **时间戳统一**：统一使用 `chrono::Utc::now().to_rfc3339()`

### 待完成
- Phase 6: 前端 Tauri 集成测试
- 提交代码到版本控制
