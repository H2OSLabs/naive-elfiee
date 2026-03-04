# Elfiee 快速启动指南

> 5 分钟快速上手 Elfiee 块编辑器

## 目录

- [前置要求](#前置要求)
- [快速安装](#快速安装)
- [启动应用](#启动应用)
- [基本操作](#基本操作)
- [常见问题](#常见问题)

---

## 前置要求

### 系统要求
- **操作系统**：Linux, macOS, 或 Windows
- **Node.js**：v18+ (推荐使用 v20)
- **pnpm**：v8+ (推荐 v10+)
- **Rust**：1.70+ (通过 rustup 安装)

### 安装工具链

```bash
# 安装 Rust (如果未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 pnpm (如果未安装)
npm install -g pnpm

# 验证安装
rustc --version  # 应显示 rustc 1.70+
pnpm --version   # 应显示 pnpm 8+
node --version   # 应显示 v18+
```

### Linux 系统依赖（重要！）

**如果你在 Linux（包括 WSL2）上开发，必须先安装系统库**：

```bash
# Ubuntu/Debian
sudo apt update

# 方案 1：自动检测并安装（推荐）
# 搜索可用的 webkit 包
apt-cache search webkit2gtk | grep dev

# 根据搜索结果安装，常见包名：
# - libwebkit2gtk-4.1-dev (Ubuntu 22.04+)
# - libwebkit2gtk-4.0-dev (Ubuntu 20.04)

# 方案 2：完整安装命令（Ubuntu 22.04+）
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

# 方案 3：如果方案 2 失败，尝试旧版本包名（Ubuntu 20.04）
sudo apt install -y \
  libwebkit2gtk-4.0-dev \
  build-essential \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

# 验证安装
pkg-config --modversion glib-2.0      # 应显示 2.70+
pkg-config --modversion gtk+-3.0      # 应显示 3.x
pkg-config --modversion webkit2gtk-4.0 || \
pkg-config --modversion webkit2gtk-4.1  # 应显示 2.x
```

**常见错误**：
```
error: failed to run custom build command for `glib-sys`
The system library `glib-2.0` required by crate `glib-sys` was not found.
```

**解决方案**：按上面的命令安装系统依赖即可。

### WSL2 特别说明

如果你在 **Windows WSL2** 中开发：

1. **确保使用 WSLg（Windows 11）或配置 X Server（Windows 10）**
   ```bash
   # Windows 11 自带 WSLg，无需配置

   # Windows 10 需要安装 VcXsrv 或 X410，并设置：
   export DISPLAY=:0
   ```

2. **项目位置建议**
   - ✅ 推荐：放在 Linux 文件系统 (`/home/yaosh/projects/`)
   - ❌ 避免：放在 Windows 文件系统 (`/mnt/c/Users/...`)
   - 原因：跨文件系统性能差，编译会很慢

3. **验证 GUI 支持**
   ```bash
   # 测试 GTK 应用能否显示
   sudo apt install x11-apps
   xeyes  # 应该弹出一个眼睛窗口
   ```

---

## 快速安装

### 1. 克隆仓库

```bash
git clone https://github.com/your-org/elfiee.git
cd elfiee
```

### 2. 安装依赖

```bash
# 安装前端依赖
pnpm install

# ⚠️ 可能会看到构建脚本警告（可以忽略）
# Warning: Ignored build scripts: esbuild, msw.
# 这是 pnpm v10+ 的安全特性，不影响使用
# 如需解决：运行 pnpm approve-builds

# 编译后端 (首次会花费较长时间，5-10 分钟)
cd src-tauri
cargo build
cd ..
```

**可能遇到的警告**：

```
╭ Warning ───────────────────────────────────────────────────────╮
│ Ignored build scripts: esbuild, msw.                           │
│ Run "pnpm approve-builds" to pick which dependencies should    │
│ be allowed to run scripts.                                     │
╰────────────────────────────────────────────────────────────────╯
```

**不用担心**：这是安全特性，不影响开发。可以选择：
- **忽略**（推荐）：依赖已正确安装
- **批准**：运行 `pnpm approve-builds` 并选择信任的包

### 3. 验证安装

```bash
# 后端测试
cd src-tauri
cargo test     # 应显示 "60 tests passed"
cd ..

# 前端类型检查（正确命令）
npx tsc --noEmit   # 不是 pnpm run type-check

# 或者运行构建（包含类型检查）
pnpm build

# 查看可用的脚本
pnpm run  # 列出所有可用命令
```

---

## 启动应用

### 开发模式

```bash
# 启动开发服务器 (热重载)
pnpm tauri dev
```

这将：
1. 启动 Vite 开发服务器 (前端热重载)
2. 编译 Rust 后端
3. 自动生成 TypeScript 类型绑定 (`src/bindings.ts`)
4. 打开 Tauri 应用窗口

### 生产构建

```bash
# 构建生产版本
pnpm tauri build

# 构建产物位置：
# Linux:   src-tauri/target/release/bundle/appimage/
# macOS:   src-tauri/target/release/bundle/dmg/
# Windows: src-tauri/target/release/bundle/msi/
```

---

## 基本操作

### 第一次使用

1. **创建新的 .elf 文件**
   - 点击工具栏的 **"New File"** 按钮
   - 文件会自动保存到临时目录

2. **创建第一个区块**
   - 点击 **"Create Block"** 按钮
   - 输入区块名称（如 "我的第一个笔记"）
   - 选择区块类型（默认 "markdown"）

3. **编辑内容**
   - 点击区块右侧的 **"Edit"** 按钮
   - 在文本框中输入 Markdown 内容
   - 点击 **"Save"** 保存修改

4. **保存文件**
   - 点击工具栏的 **"Save As"** 按钮
   - 选择保存位置（如 `~/Documents/my-notes.elf`）

### 核心概念速览

#### 区块 (Block)
- 内容的基本单元
- 每个区块有唯一 ID、名称、类型和内容
- 支持多种类型：Markdown、代码、图表等

#### 事件 (Event)
- 所有修改都记录为不可变事件
- 可以查看完整历史（点击 **"Event Viewer"** 标签）
- 支持审计和时间旅行

#### 权限 (Permission)
- 每个区块有所有者
- 可以授予其他编辑者权限（点击 **"Permissions"** 标签）
- 基于能力的访问控制 (CBAC)

---

## 常用操作示例

### 创建链接区块

```
1. 创建两个区块 A 和 B
2. 点击 "Links" 标签
3. 选择源区块 A
4. 选择目标区块 B
5. 输入关系类型（如 "references"）
6. 点击 "Add Link"
```

### 查看事件历史

```
1. 点击顶部 "Events" 标签
2. 查看所有事件列表
3. 每个事件显示：
   - 实体 ID (Entity)
   - 属性 (Attribute = editor_id/cap_id)
   - 值 (Value)
   - 时间戳 (Timestamp)
```

### 授予权限

```
1. 点击 "Permissions" 标签
2. 选择目标区块
3. 选择编辑者
4. 选择能力（如 "markdown.write"）
5. 点击 "Grant Permission"
```

---

## 常见问题

### Q1: Linux 上编译失败：`glib-sys` 或 `gobject-sys` 错误

**错误信息**：
```
error: failed to run custom build command for `glib-sys v0.18.1`
The system library `glib-2.0` required by crate `glib-sys` was not found.
```

**解决方案**：安装系统依赖

```bash
# Ubuntu 22.04+
sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev build-essential

# Ubuntu 20.04
sudo apt install -y libwebkit2gtk-4.0-dev libgtk-3-dev build-essential

# 验证
pkg-config --modversion glib-2.0
```

详见上面的 [Linux 系统依赖](#linux-系统依赖重要) 部分。

---

### Q2: pnpm 安装警告：`Ignored build scripts: esbuild, msw`

**警告信息**：
```
╭ Warning ───────────────────────────────────────╮
│ Ignored build scripts: esbuild, msw.          │
╰────────────────────────────────────────────────╯
```

**解决方案**：

**选项 1（推荐）**：忽略，不影响开发
```bash
# 依赖已成功安装，可以正常使用
pnpm tauri dev  # 应该能正常启动
```

**选项 2**：批准构建脚本
```bash
pnpm approve-builds
# 在交互界面中选择 esbuild 和 msw，按回车确认
```

**选项 3**：禁用警告
```bash
echo "enable-pre-post-scripts=true" >> .npmrc
```

---

### Q3: `pnpm run type-check` 报错：`Missing script`

**错误信息**：
```
ERR_PNPM_NO_SCRIPT  Missing script: type-check
```

**解决方案**：这个脚本不存在，使用正确的命令

```bash
# 正确的类型检查命令
npx tsc --noEmit

# 或者运行 build（包含类型检查）
pnpm build

# 查看所有可用脚本
pnpm run
```

**实际可用的脚本**：
```bash
pnpm dev              # 启动 Vite 开发服务器
pnpm build            # 构建（包含 tsc 类型检查）
pnpm tauri dev        # 启动 Tauri 应用
pnpm tauri build      # 生产构建
pnpm test             # 运行测试
pnpm format           # 格式化代码
pnpm format:check     # 检查代码格式
```

---

### Q4: 首次启动时编译很慢？

**A**: 正常现象。Rust 首次编译会下载并编译所有依赖，通常需要 **5-10 分钟**。后续启动会快得多（几秒钟）。

**加速技巧**：
```bash
# 使用更快的链接器（可选）
cargo install -f cargo-binutils
rustup component add llvm-tools-preview
```

---

### Q5: WSL2 中应用窗口无法显示？

**症状**：运行 `pnpm tauri dev` 后没有窗口弹出

**解决方案**：

**Windows 11（推荐）**：
```bash
# 已内置 WSLg，应该自动工作
# 如果不行，更新 WSL：
# 在 PowerShell 中运行：
wsl --update
```

**Windows 10**：
```bash
# 1. 安装 X Server (VcXsrv 或 X410)
# 2. 启动 X Server
# 3. 设置环境变量
export DISPLAY=:0

# 4. 测试
sudo apt install x11-apps
xeyes  # 应该弹出窗口
```

---

### Q6: 修改了 Rust 代码，前端类型没更新？

**A**: 重新运行 `pnpm tauri dev` 会自动重新生成 `src/bindings.ts`。

**注意**：
- ✅ `src/bindings.ts` 是**自动生成**的，不要手动编辑
- ✅ 每次运行 `pnpm tauri dev` 都会重新生成
- ✅ 如果修改了 Rust 类型定义，必须重启开发服务器

---

### Q7: 如何在多台设备间同步 .elf 文件？

**A**: `.elf` 文件是标准的 ZIP 归档，可以通过以下方式同步：
- **Git**（推荐用于文本内容）：`.elf` 文件可以直接提交
- **云存储**：Dropbox, Google Drive, OneDrive
- **自建服务器**：WebDAV, Nextcloud

---

### Q8: 支持实时协作吗？

**A**: 当前 MVP 版本**不支持**。实时协作在 [未来计划](./plan.md#实时协作事件广播) 中（需要事件广播机制）。

---

### Q9: 如何查看项目使用的依赖版本？

```bash
# 前端依赖
pnpm list

# 后端依赖
cd src-tauri
cargo tree

# 查看特定包
pnpm list react
cargo tree | grep tokio
```

---

### Q10: 编译错误：`linker 'cc' not found`

**解决方案**：安装构建工具链

```bash
# Ubuntu/Debian
sudo apt install build-essential

# 验证
gcc --version
```

---

## 下一步

### 学习更多

- **项目介绍**：阅读 [`intro.md`](./intro.md) 了解架构设计
- **详细文档**：查看 [`../README.md`](../README.md) 获取完整文档索引
- **扩展开发**：参考 [`../guides/EXTENSION_DEVELOPMENT.md`](../guides/EXTENSION_DEVELOPMENT.md) 创建自定义区块类型

### 开发指南

- **前端开发**：阅读 [`../guides/FRONTEND_DEVELOPMENT.md`](../guides/FRONTEND_DEVELOPMENT.md)
- **后端开发**：查看 Rust 代码注释和测试用例
- **测试**：运行 `cd src-tauri && cargo test` 查看完整测试套件

### 示例项目

```bash
# 查看 Markdown 扩展实现
cat src-tauri/src/extensions/markdown/mod.rs

# 查看核心能力定义
cat src-tauri/src/capabilities/builtins/create.rs

# 查看事件存储实现
cat src-tauri/src/engine/event_store.rs
```

---

## 性能指标

- **命令处理速度**：约 3,700 命令/秒
- **命令延迟**：约 270μs (包括数据库写入)
- **内存占用**：约 1-2MB/文件 (1000 个区块)
- **文件大小**：约 50-100KB/100 个区块

---

## 快速故障排查表

| 问题 | 症状 | 快速解决 |
|------|------|----------|
| **Linux 编译失败** | `glib-sys` 错误 | `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev` |
| **pnpm 警告** | `Ignored build scripts` | 忽略（安全特性，不影响使用） |
| **类型检查失败** | `Missing script: type-check` | 使用 `npx tsc --noEmit` |
| **WSL2 无窗口** | 运行后无界面 | Windows 11: `wsl --update` |
| **编译慢** | 首次 5-10 分钟 | 正常，后续会快 |
| **链接器错误** | `linker 'cc' not found` | `sudo apt install build-essential` |
| **bindings 不更新** | 类型定义旧 | 重启 `pnpm tauri dev` |

---

## 开发环境检查清单

在开始开发前，确保以下都正常：

```bash
# ✅ 工具链版本
rustc --version   # 1.70+
node --version    # v18+
pnpm --version    # 8+

# ✅ Linux 系统库（仅 Linux/WSL2）
pkg-config --modversion glib-2.0      # 2.70+
pkg-config --modversion gtk+-3.0      # 3.x
pkg-config --modversion webkit2gtk-4.1  # 2.x

# ✅ 依赖安装
ls node_modules/react      # 应存在
ls src-tauri/target/       # 应存在

# ✅ 测试通过
cd src-tauri && cargo test  # 60 tests passed
cd .. && npx tsc --noEmit   # No errors

# ✅ 能够启动
pnpm tauri dev  # 应弹出窗口
```

---

## 获取帮助

- **问题反馈**：提交 GitHub Issue
- **功能建议**：提交 Feature Request
- **贡献代码**：阅读 CONTRIBUTING.md (如有)
- **社区讨论**：查看项目 Discussions 板块

---

**上次更新**：2025-10-29（更新安装指南和故障排查）
**版本**：MVP 1.0 (100% 完成)
**测试环境**：Ubuntu 22.04 (WSL2), pnpm 10.19.0, Rust 1.70+
