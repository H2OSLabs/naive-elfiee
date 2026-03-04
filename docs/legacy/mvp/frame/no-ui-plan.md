# No-UI 重构计划 — 纯后端项目

## Context

Tauri 前端依赖已从 Rust 代码中移除（lib.rs、commands/、events.rs、Cargo.toml 中的 tauri/specta 依赖），但项目结构仍保留 Tauri 时代的 `src/`（React 前端）和 `src-tauri/`（Rust 后端）分层。

**目标**：变成标准纯 Rust 项目结构，根目录直接有 `Cargo.toml`、`src/`、`tests/`。

---

## Step 1: 删除前端文件和目录

**目录：**
- `src/` — React 前端代码（App.tsx, components/, pages/ 等）
- `node_modules/` — npm 依赖
- `dist/` — Vite 构建产物
- `.cursor/` — 只有 shadcn MCP 配置

**文件：**
- `package.json`, `pnpm-lock.yaml`
- `tsconfig.json`, `tsconfig.node.json`
- `vite.config.ts`, `tailwind.config.ts`, `postcss.config.js`
- `index.html`, `components.json`
- `.prettierrc`, `.prettierignore`

**文档：**
- `docs/guides/FRONTEND_DEVELOPMENT.md`

---

## Step 2: 移动 src-tauri/ 内容到根目录

`src-tauri/` 当前内容：`Cargo.toml`, `Cargo.lock`, `src/`, `tests/`, `templates/`, `capability-macros/`, `capabilities/`

```bash
# Step 1 已删除前端 src/，现在可以安全移动
mv src-tauri/Cargo.toml src-tauri/Cargo.lock .
mv src-tauri/src .
mv src-tauri/tests .
mv src-tauri/templates .
mv src-tauri/capability-macros .
mv src-tauri/capabilities .
# .elfignore 和 .elftypes 已在根目录
# 删除旧 target 目录（让 cargo 重新编译）
rm -rf src-tauri/target target
rm -rf src-tauri/
```

**`include_str!` 路径不需要修改**：源文件从 `src-tauri/src/cli/run.rs` 变为 `src/cli/run.rs`，`../../` 仍指向项目根，templates/ 也从 `src-tauri/templates/` 移到了根目录 `templates/`，相对关系完全不变。

---

## Step 3: 修改配置文件

### 3.1 `Makefile` — 重写为纯 Rust

```makefile
.PHONY: test clippy clean fmt help
.DEFAULT_GOAL := help

test:
	cargo test

clippy:
	cargo clippy

clean:
	rm -rf target/

fmt:
	cargo fmt --all

help:
	@echo "  test   - 运行测试"
	@echo "  clippy - 运行 clippy"
	@echo "  clean  - 清理构建产物"
	@echo "  fmt    - 格式化代码"
```

### 3.2 `.github/workflows/pr-tests.yml` — 简化 CI

- 移除：pnpm/Node.js 安装、前端依赖安装、Linux Tauri 依赖（webkit2gtk 等）
- 修改：`workspaces: 'src-tauri -> target'` → `'. -> target'`
- 修改：`make test` → `cargo test`

### 3.3 `.gitignore` — 清理前端 patterns

移除 React/Vite/node_modules/pnpm 相关的 ignore 规则，保留 Rust 和通用规则。

---

## Step 4: 更新 CLAUDE.md

- 项目描述：移除 "Tauri desktop app"，改为 "CLI + MCP headless server"
- 技术栈：移除 React/TypeScript/Vite
- 架构图：移除 Tauri IPC 层，保留 MCP Server + CLI 两层
- 文件路径：所有 `src-tauri/src/` → `src/`，`src-tauri/tests/` → `tests/`
- 移除 TypeScript 绑定生成规则段落
- 移除 `pnpm tauri dev/build` 命令
- 移除前端开发指南引用
- 移除 tauri/tauri-specta 从 Key Crates

---

## Step 5: 验证

1. `cargo test` — 所有测试通过
2. `cargo clippy` — 零警告
3. `cargo build --bin elf` — CLI 可构建
4. 确认根目录结构干净：

```
elfiee/
├── Cargo.toml          # 主 crate
├── Cargo.lock
├── src/                # Rust 源码
│   ├── lib.rs
│   ├── bin/elf.rs
│   ├── cli/
│   ├── engine/
│   ├── models/
│   ├── services/
│   ├── extensions/
│   ├── mcp/
│   └── ...
├── tests/              # 集成测试
├── templates/          # Workflow/Skill 模板
├── capability-macros/  # proc-macro crate
├── .elfignore          # 扫描忽略规则
├── .elftypes           # 扩展名→block type 映射
├── docs/               # 文档
└── CLAUDE.md
```

---

## 变更总结

| 操作 | 目标 |
|------|------|
| **删除** | `src/`(前端), `node_modules/`, `dist/`, `.cursor/`, 11 个前端配置文件, `docs/guides/FRONTEND_DEVELOPMENT.md` |
| **移动** | `src-tauri/*` → 项目根目录，然后删除 `src-tauri/` |
| **修改** | `Makefile`, `.github/workflows/pr-tests.yml`, `.gitignore`, `CLAUDE.md` |
| **不改** | `.claude/` 相关配置（lsp.json, .mcp.json, start-claude.sh, skills/） |

## 注意事项

- Claude 相关配置（`.claude/`、`.mcp.json`、`start-claude.sh`）暂不修改
- `elfiee-ext-gen/` 是独立 Rust 工具，保留不动（内部文档中的 `src-tauri` 引用后续更新）
- `GEMINI.md` 保留（与前端无关）
