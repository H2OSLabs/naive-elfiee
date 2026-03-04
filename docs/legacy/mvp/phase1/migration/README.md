# 功能模块迁移方案索引

## 文档信息

- **文档版本**：1.0
- **最后更新**：2025-12-12
- **总预计工时**：119 人时

---

## 迁移方案文档列表

| 序号 | 功能模块 | 文档链接 | 预计工时 | 状态 |
|------|---------|---------|---------|------|
| 01 | .elf 文件管理 | [01-elf-file-management.md](./01-elf-file-management.md) | 13 | 📝 待开发 |
| 02 | Directory Extension | [02-directory-extension.md](./02-directory-extension.md) | 30 | 📝 待开发 |
| 03 | Markdown Extension | [03-markdown-extension.md](./03-markdown-extension.md) | 12 | 📝 待开发 |
| 04 | Block 数据结构 | [04-block-data-structure.md](./04-block-data-structure.md) | 17 | 📝 待开发 |
| 05 | 协作者和权限 | [05-collaborator-permission.md](./05-collaborator-permission.md) | 6 | 📝 待开发 |
| 06 | Event 模块 | [06-event-module.md](./06-event-module.md) | 32 | 📝 待开发 |

---

## 功能模块详细列表

### 1. .elf 文件管理（13 人时）

**功能列表**：
- .elf文件CRUD：创建(1h)、导入(1h)、重命名(3h)、删除(2h)
- .elf文件展示：获取信息(2h)、展示名称/协作者/路径(3h)、dashboard页面(4h)

**关键任务**：
- 后端：实现 `rename_file` 和 `get_file_info` 命令
- 前端：创建 Dashboard 页面和相关组件

**文档**：[01-elf-file-management.md](./01-elf-file-management.md)

---

### 2. Directory Extension（30 人时）

**功能列表**：
- 基础能力：导入(4h)、创建(2h)、删除(2h)、重命名(2h)、刷新(4h)、导出(4h)
- 页面目录栏：内部目录(6h)、外部目录(6h)

**关键任务**：
- 后端：创建目录管理能力（create, rename, delete）和相关命令
- 前端：创建目录树组件和操作工具栏

**文档**：[02-directory-extension.md](./02-directory-extension.md)

---

### 3. Markdown Extension（12 人时）

**功能列表**：
- md编辑：数据格式修改(6h)
- md显示：myst语法和适配(6h)

**关键任务**：
- 后端：确认现有 `markdown.write` 和 `markdown.read` 能力
- 前端：实现 MyST 语法解析和渲染组件

**文档**：[03-markdown-extension.md](./03-markdown-extension.md)

---

### 4. Block 数据结构（17 人时）

**功能列表**：
- 数据结构修改：title/owner/description/time(4h)、引擎修改(6h)
- 显示信息：payload修改(3h)、info页面修改(4h)

**关键任务**：
- 后端：扩展 Block 模型，添加新字段
- 前端：创建 BlockInfoPanel 组件

**文档**：[04-block-data-structure.md](./04-block-data-structure.md)

---

### 5. 协作者和权限（6 人时）

**功能列表**：
- 基础功能：协作者增删(1h)、权限管理(1h)
- 权限显示：权限页面修改(4h)

**关键任务**：
- 后端：确认现有 `core.grant` 和 `core.revoke` 能力
- 前端：创建权限管理组件

**文档**：[05-collaborator-permission.md](./05-collaborator-permission.md)

---

### 6. Event 模块（32 人时）

**功能列表**：
- event解析：操作人员/名称/内容(4h)、event排序(7h)
- event回溯：回溯功能(7h)、快照功能(optional, 6h)
- 显示和交互：查看列表(6h)、回溯操作(6h)、对比显示(optional, 2h)

**关键任务**：
- 后端：实现 `get_block_content_at_event` 命令
- 前端：创建 Timeline 和 Event 显示组件

**文档**：[06-event-module.md](./06-event-module.md)

---

## 开发顺序建议

### 第一阶段：基础功能（31 人时）

1. **.elf 文件管理**（13h）- 基础文件操作
2. **协作者和权限**（6h）- 权限系统
3. **Block 数据结构**（17h）- 数据模型扩展

### 第二阶段：编辑功能（42 人时）

4. **Markdown Extension**（12h）- 编辑和显示
5. **Directory Extension**（30h）- 目录管理

### 第三阶段：高级功能（32 人时）

6. **Event 模块**（32h）- 事件记录和回溯

---

## 通用开发流程

每个功能模块的开发都应遵循以下流程：

1. **后端开发**：
   - 定义命令或能力
   - 在 `lib.rs` 中注册
   - 运行 `cargo run` 生成 `bindings.ts`
   - 编写单元测试

2. **前端开发**：
   - 查看 `bindings.ts` 了解接口
   - 在 `tauri-client.ts` 中封装
   - 在 `app-store.ts` 中添加状态管理（如需要）
   - 创建组件
   - 编写组件测试

---

## 参考文档

- [迁移标准方案](../migration-standard.md) - 前后端对接规范
- [用户故事文档](../demo/user-story-mvp.md) - 功能需求说明

---

## 文档维护

- 每个迁移方案文档都应包含完整的开发检查清单
- 开发完成后，更新文档状态为"✅ 已完成"
- 如遇到问题，在文档中记录解决方案

---

**最后更新**：2025-12-12

