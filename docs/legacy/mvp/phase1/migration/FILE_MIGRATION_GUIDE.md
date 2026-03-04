# 前端文件迁移清单

## 文档信息

- **文档版本**：1.0
- **最后更新**：2025-12-12
- **迁移方向**：`elfiee-mvp-ui` → `elfiee`

---

## 一、elfiee-mvp-ui 需要迁移的文件

### 1. 源代码文件（src/）

#### ✅ 必须迁移

**组件目录（src/components/）**：
```
src/components/
├── dashboard/              # ✅ 全部迁移
│   ├── Sidebar.tsx
│   └── TopBar.tsx
├── editor/                 # ✅ 全部迁移
│   ├── AgentContext.tsx    # ⚠️ 需要适配（Persona → Editor）
│   ├── BlockEditor.tsx
│   ├── ContextPanel.tsx
│   ├── EditorCanvas.tsx    # ✅ 核心编辑器组件
│   ├── EditorSidebar.tsx
│   ├── FilePanel.tsx
│   ├── ImportRepositoryModal.tsx
│   ├── myst-styles.css     # ✅ MyST 样式
│   ├── OutlineTree.tsx
│   └── ProjectExplorer.tsx
├── projects/               # ✅ 全部迁移
│   ├── CreateProjectModal.tsx
│   ├── ImportProjectModal.tsx
│   ├── InviteModal.tsx
│   ├── LocationBreadcrumb.tsx
│   └── ProjectCard.tsx
├── ui/                     # ✅ 全部迁移（shadcn/ui 组件）
│   └── [所有 .tsx 文件]     # 约 50+ 个 UI 组件
└── NavLink.tsx            # ✅ 迁移
```

**页面目录（src/pages/）**：
```
src/pages/
├── DocumentEditor.tsx      # ✅ 迁移
├── NotFound.tsx            # ✅ 迁移
└── Projects.tsx            # ✅ 迁移
```

**工具和库（src/lib/）**：
```
src/lib/
├── utils.ts                # ✅ 迁移（工具函数）
├── mockStore.tsx           # ❌ 不迁移（使用 elfiee 的 app-store.ts）
└── projectData.ts          # ❌ 不迁移（Mock 数据）
```

**Hooks（src/hooks/）**：
```
src/hooks/
├── use-mobile.tsx          # ✅ 迁移
└── use-toast.ts            # ✅ 迁移（如果 elfiee 没有）
```

**样式文件**：
```
src/
├── App.css                 # ✅ 迁移
├── index.css               # ✅ 迁移（合并或替换）
└── vite-env.d.ts          # ✅ 迁移
```

**入口文件**：
```
src/
├── App.tsx                 # ✅ 迁移（需要适配路由）
└── main.tsx                # ✅ 迁移（需要适配 Tauri）
```

---

### 2. 配置文件

#### ✅ 需要迁移/合并

**Tailwind 配置**：
```
tailwind.config.ts          # ✅ 迁移（合并配置）
```

**组件配置**：
```
components.json             # ✅ 迁移（更新路径）
```

**TypeScript 配置**：
```
tsconfig.json               # ✅ 参考合并
tsconfig.app.json           # ✅ 参考合并
tsconfig.node.json          # ✅ 参考合并
```

**Vite 配置**：
```
vite.config.ts              # ⚠️ 需要合并（保留 Tauri 配置）
```

**其他配置**：
```
postcss.config.js           # ✅ 迁移
.prettierrc                  # ✅ 迁移（如果不同）
.eslintrc.js / eslint.config.js  # ✅ 参考合并
```

---

### 3. 静态资源（public/）

```
public/
├── favicon.ico             # ✅ 迁移
├── placeholder.svg         # ✅ 迁移（如果使用）
└── robots.txt              # ❌ 不迁移（Tauri 不需要）
```

---

### 4. 根目录文件

```
index.html                  # ✅ 迁移（更新标题和 meta）
package.json                # ⚠️ 需要合并依赖
```

---

## 二、elfiee 需要删除的前端文件

### 1. 源代码文件（src/）

#### ❌ 需要删除

**组件目录（src/components/）**：
```
src/components/
├── BlockEditor.tsx         # ❌ 删除（使用 mvp-ui 的 EditorCanvas.tsx）
├── BlockList.tsx           # ❌ 删除（使用 mvp-ui 的组件）
├── BlockList.test.tsx      # ❌ 删除
├── BlockTypeDialog.tsx     # ❌ 删除（使用 mvp-ui 的 UI 组件）
├── BlockTypeDialog.test.tsx # ❌ 删除
├── EditorSelector.tsx      # ❌ 删除（使用 mvp-ui 的组件）
├── EditorSelector.test.tsx # ❌ 删除
├── EventViewer.tsx        # ❌ 删除（使用 mvp-ui 的 ContextPanel.tsx）
├── LinkManager.tsx        # ❌ 删除（使用 mvp-ui 的组件）
├── PermissionManager.tsx   # ❌ 删除（使用 mvp-ui 的组件）
├── Terminal.tsx           # ⚠️ 保留（如果 mvp-ui 没有）
├── Toolbar.tsx            # ❌ 删除（使用 mvp-ui 的组件）
├── Toolbar.test.tsx       # ❌ 删除
└── ui/                     # ⚠️ 部分保留（合并 mvp-ui 的 ui 组件）
    ├── button.tsx          # ⚠️ 保留（如果 mvp-ui 有更新版本）
    ├── button.test.tsx     # ❌ 删除
    ├── input.tsx           # ⚠️ 保留（如果 mvp-ui 有更新版本）
    ├── select.tsx          # ⚠️ 保留（如果 mvp-ui 有更新版本）
    ├── sonner.tsx          # ⚠️ 保留（如果 mvp-ui 有更新版本）
    ├── table.tsx           # ⚠️ 保留（如果 mvp-ui 有更新版本）
    └── toaster.tsx         # ⚠️ 保留（如果 mvp-ui 有更新版本）
```

**样式文件**：
```
src/
├── App.css                 # ❌ 删除（使用 mvp-ui 的）
└── index.css               # ⚠️ 合并（保留 Tauri 特定样式）
```

**入口文件**：
```
src/
├── App.tsx                 # ❌ 删除（使用 mvp-ui 的）
└── main.tsx                # ⚠️ 合并（保留 Tauri 初始化代码）
```

**资源文件**：
```
src/assets/
└── react.svg               # ❌ 删除（如果不需要）
```

---

### 2. 测试文件（src/test/）

```
src/test/
├── setup.ts                # ❌ 删除（后续做详细功能时在一条一条新增）
├── mock-tauri-invoke.ts    # ❌ 删除（后续做详细功能时在一条一条新增）
├── terminal-integration.test.ts  # ❌ 删除（后续做详细功能时在一条一条新增）
└── Terminal-save.test.tsx  # ❌ 删除（后续做详细功能时在一条一条新增）
```

---

### 3. 静态资源（public/）

```
public/
├── tauri.svg               # ❌ 删除（如果不需要）
└── vite.svg                # ❌ 删除（使用 mvp-ui 的 favicon）
```

---

## 三、elfiee 需要保留/更新的文件

### ✅ 必须保留

**后端自动生成**：
```
src/bindings.ts             # ✅ 保留（后端自动生成，只读）
```

**前端核心封装**：
```
src/lib/
├── tauri-client.ts         # ❌ 删除（后续做详细功能时在一条一条新增）
├── app-store.ts            # ❌ 删除（后续做详细功能时在一条一条新增）
├── constants.ts            # ❌ 删除（后续做详细功能时在一条一条新增）
├── utils.ts                # ❌ 删除（后续做详细功能时在一条一条新增）
└── *.test.ts               # ❌ 删除（后续做详细功能时在一条一条新增）
```

**配置文件**：
```
vite.config.ts              # ✅ 保留（合并 mvp-ui 配置）
tsconfig.json               # ✅ 保留（合并 mvp-ui 配置）
package.json                # ✅ 保留（合并依赖）
components.json             # ⚠️ 更新（使用 mvp-ui 的配置）
```

**Tauri 相关**：
```
src-tauri/                  # ✅ 全部保留（后端代码）
Makefile                    # ✅ 保留
```

---

## 四、依赖合并清单

### package.json 依赖对比

**elfiee-mvp-ui 有但 elfiee 没有的依赖**：
```json
{
  "@hookform/resolvers": "^3.10.0",
  "@myst-theme/providers": "^1.0.0",
  "@radix-ui/react-accordion": "^1.2.11",
  "@radix-ui/react-alert-dialog": "^1.1.14",
  "@radix-ui/react-aspect-ratio": "^1.1.7",
  "@radix-ui/react-avatar": "^1.1.10",
  "@radix-ui/react-checkbox": "^1.3.2",
  "@radix-ui/react-collapsible": "^1.1.11",
  "@radix-ui/react-context-menu": "^2.2.15",
  "@radix-ui/react-dialog": "^1.1.14",
  "@radix-ui/react-dropdown-menu": "^2.1.15",
  "@radix-ui/react-hover-card": "^1.1.14",
  "@radix-ui/react-label": "^2.1.7",
  "@radix-ui/react-menubar": "^1.1.15",
  "@radix-ui/react-navigation-menu": "^1.2.13",
  "@radix-ui/react-popover": "^1.1.14",
  "@radix-ui/react-progress": "^1.1.7",
  "@radix-ui/react-radio-group": "^1.3.7",
  "@radix-ui/react-scroll-area": "^1.2.9",
  "@radix-ui/react-separator": "^1.1.7",
  "@radix-ui/react-slider": "^1.3.5",
  "@radix-ui/react-switch": "^1.2.5",
  "@radix-ui/react-tabs": "^1.1.12",
  "@radix-ui/react-toast": "^1.2.14",
  "@radix-ui/react-toggle": "^1.1.9",
  "@radix-ui/react-toggle-group": "^1.1.10",
  "@radix-ui/react-tooltip": "^1.2.7",
  "@tanstack/react-query": "^5.83.0",
  "cmdk": "^1.1.1",
  "date-fns": "^3.6.0",
  "embla-carousel-react": "^8.6.0",
  "framer-motion": "^12.23.25",
  "input-otp": "^1.4.2",
  "react-day-picker": "^8.10.1",
  "react-hook-form": "^7.61.1",
  "react-resizable-panels": "^2.1.9",
  "react-router-dom": "^6.30.1",
  "react-syntax-highlighter": "^15.5.0",
  "recharts": "^2.15.4",
  "unified": "^11.0.5",
  "vaul": "^0.9.9",
  "zod": "^3.25.76",
  "myst-parser": "^1.6.3",
  "myst-to-react": "^1.0.0"
}
```

**elfiee 有但 mvp-ui 没有的依赖**：
```json
{
  "@tauri-apps/api": "^2",
  "@tauri-apps/plugin-dialog": "^2.4.0",
  "@tauri-apps/plugin-fs": "^2",
  "@tauri-apps/plugin-opener": "^2",
  "@xterm/addon-fit": "^0.10.0",
  "@xterm/addon-web-links": "^0.11.0",
  "@xterm/xterm": "^5.5.0",
  "zustand": "^5.0.8"
}
```

**需要合并的依赖**（版本可能不同）：
```json
{
  "@radix-ui/react-select": "^2.2.6" vs "^2.2.5",
  "@radix-ui/react-slot": "^1.2.3" (相同),
  "class-variance-authority": "^0.7.1" (相同),
  "clsx": "^2.1.1" (相同),
  "lucide-react": "^0.546.0" vs "^0.462.0",
  "next-themes": "^0.4.6" vs "^0.3.0",
  "react": "^18.3.1" (相同),
  "react-dom": "^18.3.1" (相同),
  "sonner": "^2.0.7" vs "^1.7.4",
  "tailwind-merge": "^3.3.1" vs "^2.6.0"
}
```

---

## 五、迁移步骤

### 阶段 1：准备阶段

1. **备份 elfiee 项目**
   ```bash
   cd elfiee
   git commit -am "备份：迁移前的前端代码"
   ```

2. **创建迁移分支**
   ```bash
   git checkout -b feature/migrate-mvp-ui-frontend
   ```

### 阶段 2：删除旧文件

1. **删除旧组件**
   ```bash
   rm -rf src/components/BlockEditor.tsx
   rm -rf src/components/BlockList.*
   rm -rf src/components/BlockTypeDialog.*
   rm -rf src/components/EditorSelector.*
   rm -rf src/components/EventViewer.tsx
   rm -rf src/components/LinkManager.tsx
   rm -rf src/components/PermissionManager.tsx
   rm -rf src/components/Toolbar.*
   ```

2. **删除旧样式**
   ```bash
   rm src/App.css  # 备份后删除
   ```

3. **删除旧入口文件**
   ```bash
   # 备份后删除
   mv src/App.tsx src/App.tsx.backup
   mv src/main.tsx src/main.tsx.backup
   ```

### 阶段 3：迁移新文件

1. **复制组件**
   ```bash
   cp -r ../elfiee-mvp-ui/src/components/dashboard src/components/
   cp -r ../elfiee-mvp-ui/src/components/editor src/components/
   cp -r ../elfiee-mvp-ui/src/components/projects src/components/
   cp -r ../elfiee-mvp-ui/src/components/NavLink.tsx src/components/
   ```

2. **合并 UI 组件**
   ```bash
   # 合并 ui 目录，保留测试文件
   cp -r ../elfiee-mvp-ui/src/components/ui/* src/components/ui/
   ```

3. **复制页面**
   ```bash
   mkdir -p src/pages
   cp -r ../elfiee-mvp-ui/src/pages/* src/pages/
   ```

4. **复制工具和 hooks**
   ```bash
   cp ../elfiee-mvp-ui/src/lib/utils.ts src/lib/utils.ts.mvp
   # 手动合并 utils.ts
   cp -r ../elfiee-mvp-ui/src/hooks src/
   ```

5. **复制样式**
   ```bash
   cp ../elfiee-mvp-ui/src/App.css src/
   cp ../elfiee-mvp-ui/src/components/editor/myst-styles.css src/components/editor/
   # 合并 index.css
   ```

6. **复制入口文件**
   ```bash
   cp ../elfiee-mvp-ui/src/App.tsx src/
   cp ../elfiee-mvp-ui/src/main.tsx src/
   # 需要手动适配 Tauri
   ```

### 阶段 4：更新配置

1. **合并 package.json**
   - 合并所有依赖
   - 保留 Tauri 相关脚本
   - 保留测试相关脚本

2. **合并 vite.config.ts**
   - 保留 Tauri 配置
   - 添加 mvp-ui 的插件配置

3. **合并 tailwind.config.ts**
   - 使用 mvp-ui 的配置（更完整）

4. **更新 components.json**
   - 使用 mvp-ui 的配置

### 阶段 5：代码适配

1. **更新 App.tsx**
   - 适配 Tauri 环境
   - 移除 Persona，使用 Editor
   - 更新路由配置

2. **更新 main.tsx**
   - 保留 Tauri 初始化代码
   - 更新主题提供者

3. **更新组件**
   - 替换 `mockStore` 为 `app-store`
   - 替换 `Persona` 为 `Editor`
   - 更新 API 调用为 `tauri-client`

4. **更新样式**
   - 合并 CSS 变量
   - 确保主题一致

### 阶段 6：测试和修复

1. **安装依赖**
   ```bash
   pnpm install
   ```

2. **运行开发服务器**
   ```bash
   pnpm tauri dev
   ```

3. **修复错误**
   - TypeScript 类型错误
   - 导入路径错误
   - 缺失的依赖

4. **运行测试**
   ```bash
   pnpm test
   ```

---

## 六、注意事项

### ⚠️ 重要提醒

1. **bindings.ts 不要修改**
   - 这是后端自动生成的，运行 `cargo run` 会自动更新

2. **tauri-client.ts 需要更新**
   - 可能需要添加新的方法以支持 mvp-ui 的组件

3. **app-store.ts 需要更新**
   - 可能需要添加新的状态管理方法

4. **路由配置**
   - mvp-ui 使用 `react-router-dom`
   - 需要确认 Tauri 应用的路由策略

5. **主题系统**
   - mvp-ui 使用 `next-themes`
   - 需要确保与 Tauri 兼容

6. **MyST 解析**
   - mvp-ui 使用 `myst-parser` 和 `myst-to-react`
   - 需要确保依赖正确安装

---

## 七、迁移检查清单

### 文件迁移

- [ ] 组件文件已迁移
- [ ] 页面文件已迁移
- [ ] UI 组件已合并
- [ ] 样式文件已迁移
- [ ] 工具函数已合并
- [ ] Hooks 已迁移
- [ ] 配置文件已合并
- [ ] 静态资源已迁移

### 代码适配

- [ ] App.tsx 已适配 Tauri
- [ ] main.tsx 已适配 Tauri
- [ ] 组件已替换 mockStore
- [ ] 组件已替换 Persona
- [ ] API 调用已更新为 tauri-client
- [ ] 路由配置已更新

### 配置更新

- [ ] package.json 依赖已合并
- [ ] vite.config.ts 已合并
- [ ] tailwind.config.ts 已更新
- [ ] components.json 已更新
- [ ] tsconfig.json 已合并

### 测试验证

- [ ] 项目可以启动
- [ ] 无 TypeScript 错误
- [ ] 无运行时错误
- [ ] 组件正常渲染
- [ ] 路由正常工作
- [ ] 主题切换正常
- [ ] Tauri 功能正常

---

**最后更新**：2025-12-12
