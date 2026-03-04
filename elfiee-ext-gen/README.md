# elfiee-ext-gen

Elfiee 扩展生成器 - 基于测试驱动开发（TDD）的代码生成工具，用于快速创建 Elfiee 项目的扩展（Extensions）。

## 功能特性

- **快速生成扩展骨架**: 自动生成扩展的完整文件结构
- **测试驱动开发**: 生成包含完整测试用例的代码，先写测试，引导实现
- **智能字段推断**: 基于能力名称推断合理的 Payload 字段，并在模板中直接给出注释示例
- **开箱即用**: 生成的代码可直接编译，测试可直接运行
- **详细开发指南**: 自动生成包含清晰 TODO 标记的开发指南，并提供分类统计帮助排障
- **一致性校验**: CLI 会校验能力名称必须为 `snake_case`，注册校验基于 AST 解析避免遗漏

## 安装

### 使用 cargo install（推荐）

```bash
# 在 elfiee 项目根目录
cargo install --path elfiee-ext-gen --force

# 验证安装
elfiee-ext-gen --version
```

安装或模板更新后，重新执行上面的命令即可刷新本地二进制。

### 开发模式（用于调试模板）

```bash
cd elfiee-ext-gen
cargo run -- create -n ext_name -b block_type -c action1
```

## 使用方法

### 命令行参数

```
elfiee-ext-gen create [OPTIONS]

选项:
  -n, --extension-name <NAME>      扩展名称（snake_case）
  -b, --block-type <TYPE>          块类型（例如：markdown, component）
  -c, --capabilities <LIST>        能力列表，逗号分隔（例如：read,write）
      --with-auth-tests            生成授权测试（默认：true）
      --with-workflow-tests        生成工作流测试（默认：true）
  -h, --help                       显示帮助信息
```

### 实际示例

#### 1. 创建完整扩展

```bash
# 创建一个 markdown 扩展，包含 read 和 write 能力
elfiee-ext-gen create \
  -n markdown \
  -b markdown \
  -c read,write
```

生成的文件结构：
```
src/extensions/markdown/
├── mod.rs                    # 模块定义和 Payload 结构
├── markdown_read.rs          # read 能力处理器
├── markdown_write.rs         # write 能力处理器
├── tests.rs                  # 完整测试套件
└── DEVELOPMENT_GUIDE.md      # 开发指南
```

#### 2. 创建仅带授权测试的扩展

```bash
# 创建一个组件扩展，保留授权测试、关闭工作流测试
elfiee-ext-gen create \
  -n markdown \
  -b markdown \
  -c read,write \
  --with-workflow-tests=false
```

#### 3. 创建简单的扩展（不包含权限和workflow测试）

```bash
# 创建一个功能完整的扩展，保留授权与工作流测试（默认即启用）
  elfiee-ext-gen create \
    -n data_store \
    -b data \
    -c save,load,query \
    --with-workflow-tests=false \
    --with-auth-tests=false
```

### 基本用法

```bash
# 在 elfiee 项目根目录运行（需要访问 src-tauri/src 下的注册文件）
cd /path/to/elfiee

# 生成扩展（假设已通过 cargo install 安装）
elfiee-ext-gen create \
  -n ext_name \
  -b block_type \
  -c render,update
```

### 推荐开发流程

1. **Create**：在项目根目录执行 `elfiee-ext-gen create ...`。
2. **Guide**：继续在根目录运行 `elfiee-ext-gen guide <extension>`，明确当前失败的测试和下一步。
   - 最新版本会在输出末尾新增 “Category Summary”，快速统计 Payload / Functionality / Authorization / Workflow 等类别的告警，方便排查。
3. **Test**：进入 `src-tauri`，执行 `cargo test <extension>::tests -- --nocapture`，根据输出实现 TODO。
4. **Validate**：回到项目根目录，运行 `elfiee-ext-gen validate <extension>` 检查结构与注册。
5. **重复 2-4 步**，直至所有测试通过，Guide 显示 100%。



## 生成的代码结构

### 模块文件 (mod.rs)

```rust
pub mod my_extension_capability1;
pub use my_extension_capability1::*;

// Payload 定义
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Capability1Payload {
    // // 建议字段示例（来自推断）
    // pub text: String,
    // pub priority: Option<u32>,
    /// TODO: 替换占位字段，或按需取消上面的注释
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests;
```

### 能力处理器 (capability.rs)

```rust
/// 能力处理器函数，已包含 #[capability] 宏
#[capability(id = "extension.capability", target = "block_type")]
fn handle_capability(
    cmd: &Command,
    block: Option<&Block>
) -> CapResult<Vec<Event>> {
    // TODO: 实现处理逻辑
    todo!("Implement capability handler");
}
```

### 测试文件 (tests.rs)

包含以下测试类型：

**1. Payload 反序列化测试**（每个能力 1 个）
- 验证 JSON 到 Payload 的转换
- 确保字段类型正确

**2. 基本功能测试**（每个能力 1 个）
- 测试能力处理器的核心逻辑
- 验证生成的事件结构
- 检查事件的 entity、attribute、value 字段

**3. 授权测试**（每个能力 3 个，默认生成）
- `test_{capability}_authorization_owner`: 验证块所有者总是被授权
- `test_{capability}_authorization_non_owner_without_grant`: 验证非所有者无授权时被拒绝
- `test_{capability}_authorization_non_owner_with_grant`: 验证非所有者获得授权后可执行
- 完整覆盖 Elfiee 的 CBAC (Capability-Based Access Control) 逻辑

**4. 工作流测试**（整个扩展 1 个，默认生成）
- 测试多个能力的交互场景（如：创建 → 更新 → 查询）
- 验证状态转换的正确性（模拟 StateProjector）
- 模拟真实使用流程的端到端测试

## 开发工作流

### 1. 生成扩展

```bash
cd /path/to/elfiee
elfiee-ext-gen create -n my_ext -b my_type -c action1,action2
```

### 2. 使用命令行指南

```bash
elfiee-ext-gen guide my_ext
```

指南会列出失败的测试、跳转位置和下一步建议；同时可以参考自动生成的 `src-tauri/src/extensions/my_ext/DEVELOPMENT_GUIDE.md`。

示例输出：

```
📊 Test Status: 50% (3/6)

🔴 Failures:

  Test: extensions::directory::tests::test_list_payload_deserialize
  Category: payload_field_missing
  Hint: Add the missing field to the payload struct definition
  Next step: 定义 payload 字段并更新示例 JSON

📚 Category Summary:
- ❌ Payload: 1 failing (extensions::directory::tests::test_list_payload_deserialize)
- ✅ Functionality: all tests passing
- ✅ Authorization: all tests passing
- ❌ Workflow: 1 failing (extensions::directory::tests::test_full_workflow)
- ✅ Types & Imports: all tests passing
- ✅ Other: all tests passing

⏱️  Estimated time: 30 minutes
```

### 3. 按 TODO 标记实现

代码与指南已经标注具体 TODO，推荐顺序：

1. **定义 Payload 字段**（`mod.rs`）
2. **实现能力处理器**（`*_*.rs`）
3. **完善测试用例**（`tests.rs`）
4. **注册到 registry / Specta**（自动生成，但可视需要调整）

### 4. 在 src-tauri 下运行测试

```bash
cd /path/to/elfiee/src-tauri
cargo test my_ext::tests -- --nocapture
```

### 5. 回到根目录执行验证

```bash
cd /path/to/elfiee
elfiee-ext-gen validate my_ext
```

验证会检查模块导出、registry 注册、Specta 类型等是否完整。


### TDD 流程示例

```bash
# 1. 生成扩展
elfiee-ext-gen create -n my_ext -b my_type -c action

# 2. 查看指南（会列出失败的测试和下一步建议）
elfiee-ext-gen guide my_ext

# 3. 进入 src-tauri，运行针对性的测试（会失败）
cd src-tauri
cargo test my_ext::tests -- --nocapture
# 输出: thread panicked at 'not yet implemented: ...'

# 4. 实现 Payload 字段
# 编辑 src/extensions/my_ext/mod.rs

# 5. 实现处理器逻辑
# 编辑 src/extensions/my_ext/my_ext_action.rs

# 6. 完善测试用例
# 编辑 src/extensions/my_ext/tests.rs

# 7. 再次运行测试并验证
cargo test my_ext::tests -- --nocapture
cd ..
elfiee-ext-gen validate my_ext
```

## 高级用法

### 自定义模板

如果需要修改生成的代码模板，可以编辑 `elfiee-ext-gen/templates/` 目录下的模板文件：

- `mod.rs.tera` - 模块文件模板
- `capability.rs.tera` - 能力处理器模板
- `tests.rs.tera` - 测试文件模板
- `DEVELOPMENT_GUIDE.md.tera` - 开发指南模板

修改后重新构建工具即可生效。

### 自定义 Guide 规则

- 在 `elfiee-ext-gen/rules/error_patterns.yaml` 中添加新的编译/测试错误正则，指定分类与提示。
- 在 `elfiee-ext-gen/rules/next_steps.yaml` 中为分类配置优先级与下一步操作。
- 若某个扩展需要额外的测试依赖关系，可在 `src-tauri/src/extensions/<extension_name>/test_dependencies.yaml` 定义自有规则，添加`guide` 自动合并项目级与扩展级配置的逻辑（未实现）。

### 自定义字段推断

- `core/generator.rs::infer_fields` 提供了基于能力名称的默认推断，可按需扩展匹配规则或重写逻辑。
- 模板 `templates/mod.rs.tera` 会根据推断结果生成注释示例，若要替换默认占位字段或新增注释格式，可直接修改模板。

### 扩展 CLI 行为

- `create`：可通过扩展 `ExtensionConfig` 字段、修改模板或 `infer_fields` 来注入更多默认逻辑。
- `guide`：除了规则拓展，还能在 `core/guide_gen.rs` 中调整输出格式（例如新增章节、接入覆盖率数据）。
- `validate`：目前默认检查模块导出与注册，可在 `core/validator.rs` 添加新的 AST 校验或扩展 Specta 注册逻辑。

### 验证现有扩展

```bash
# 在项目根目录验证现有扩展
elfiee-ext-gen validate my_ext
```

## 故障排查

- **模板路径错误**：如果在非仓库根目录运行生成器，报错信息会列出尝试过的模板目录及详细原因，检查路径后再次执行。
- **能力名称非法**：`create` 命令仅接受 `snake_case` 能力名（字母 / 数字 / 下划线，不能以数字开头）。若需其他格式，请先调整能力标识再生成。
- **注册校验失败**：`validate` 现在通过 AST 解析确认 `CapabilityRegistry::register_extensions` 中是否正确调用 `self.register(Arc::new(...))`，按提示补全 `registry.rs` 或 `lib.rs` 即可。

## 贡献指南

欢迎贡献！提交 PR 前请确认：

1. 在 `elfiee-ext-gen/` 目录运行 `cargo fmt && cargo clippy && cargo test`
2. 若改动影响生成结果，请更新模板及对应文档
3. 在 `src-tauri/` 中用新模板生成的示例扩展跑通 `cargo test <extension>::tests`

## 许可证

Apache-2.0 License

## 版本历史

### v0.1.2 (2025-11-04)

- 模板加载失败时列出所有搜索路径，便于定位环境问题
- Guide 输出新增分类统计（Payload / Functionality / Authorization / Workflow / Types），修复更有方向
- CLI `create` 校验能力名必须为 `snake_case`，避免生成非法标识符
- Validator 改用 AST 解析校验 `extensions/mod.rs` 与 `CapabilityRegistry::register_extensions`
- Payload 模板内直接给出字段推断的注释示例，提升可操作性
- 新增常见错误模式提示类型导入、类型不匹配与所有权问题

### v0.1.1 (2025-11-02)

- Guide 解析新增 payload 示例 / handler TODO / workflow TODO 等匹配规则
- 模板默认导入 `create_event`，并在注释中说明何时可以返回空事件
- README、验证器同步更新，指引以项目根目录为中心的工作流

### v0.1.0 (2025-10-31)

- 初始版本，提供扩展骨架、字段推断、测试模板与注册脚本
