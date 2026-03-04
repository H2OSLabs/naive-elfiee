# Elfiee 团队协作工作流程

**版本**: 1.0
**适用范围**: Elfiee MVP 前端开发
---

## 目录

1. [团队角色与职责](#团队角色与职责)
2. [仓库架构](#仓库架构)
3. [完整工作流程](#完整工作流程)
4. [详细操作指南](#详细操作指南)
5. [分支命名规范](#分支命名规范)
6. [质量保证机制](#质量保证机制)
7. [常见问题处理](#常见问题处理)
8. [工具和环境配置](#工具和环境配置)

---

## 团队角色与职责

### 👤 sy

**主要职责**：
- 发起需求讨论和技术决策
- 前后端开发
- Review 所有代码变更
- 负责 `elfiee-mvp-ui` → `elfiee` 的迁移
- 统筹项目进度和质量

**工作范围**：
- ✅ 技术架构设计
- ✅ 前后端开发
- ✅ 代码 Review 和合并
- ✅ 迁移脚本开发 (potential)
- ✅ 最终集成到主项目

---

### 👤 zy

**主要职责**：
- 实现前端功能和交互逻辑
- 调整 rh 提交的代码以符合技术规范
- 协助 sy 进行迁移工作
- 开发修改必须的后端功能

**工作范围**：
- ✅ 前端开发
- ✅ 代码调整（基于 rh 的设计）
- ✅ 单元测试编写和bug修复
- ✅ 后端功能开发
- ✅ 代码 Review 和合并

---

### 👤 rh（Product & Design）

**主要职责**：
- 定义 MVP 产品体验
- UI 和交互设计
- 在 Lovable 中通过 Prompt 实现设计

**工作范围**：
- ✅ 产品需求定义
- ✅ UI/UX 设计
- ✅ 在 Lovable 中生成组件
- ✅ 样式和交互调整

---

## 仓库架构

### 仓库 1: `elfiee-mvp-ui`（前端原型库）

**地址**: `git@github.com:H2OSLabs/elfiee-mvp-ui.git`

**用途**：
- 纯前端开发和设计迭代
- Lovable 同步目标
- Mock 数据驱动，无 Tauri 依赖

**技术栈**：
- React 18 + TypeScript
- TailwindCSS + shadcn/ui
- Vite
- Mock 数据（`src/data/mockElfieeData.ts`）

**分支策略**：
```
main                    ← 稳定的 UI 版本，用于迁移
├── feat/ui-*-rh        ← rh 在 Lovable 中创建的设计分支
│   ├── feat/ui-*-dev       ← zy/sy 基于 rh 分支调整的开发分支
└── hotfix/*            ← 紧急修复分支
```

**关键文件**：
```
elfiee-mvp-ui/
├── src/
│   ├── components/     ← UI 组件（Lovable 生成）
│   ├── pages/          ← 页面组件
│   ├── data/
│   │   └── mockElfieeData.ts  ← Mock 数据
│   ├── types/
│   │   └── index.ts    ← 类型定义（临时，迁移时会被 bindings.ts 替换）
│   └── hooks/          ← 前端状态管理 hooks（无 Tauri 调用）
├── .github/
│   └── workflows/
│       └── code-quality-check.yml  ← 代码质量检查
└── README.md
```

---

### 仓库 2: `elfiee`（主项目库）

**地址**: `git@github.com:H2OSLabs/elfiee.git`

**用途**：
- 完整的 Elfiee 应用（Tauri + React）
- 生产环境代码
- 集成后端 Event Store

---

## 完整工作流程

### 流程图

```
┌─────────────────────────────────────────────────────────────────┐
│                     Elfiee 开发流程                              │
└─────────────────────────────────────────────────────────────────┘

Phase 1: 需求和设计
────────────────────
[sy 发起讨论]
      ↓
[sy + rh + zy 确认需求]
      ↓
[rh 创建设计分支]
feat/ui-{feature}-rh


Phase 2: Lovable 设计实现（在 elfiee-mvp-ui）
────────────────────────────────────────────
[rh 在 Lovable 中]
- 编写 Prompt
- 生成组件
- 调整样式
      ↓
[Lovable 自动推送到 GitHub]
feat/ui-{feature}-rh 分支更新
      ↓
[rh 通知 zy/sy]
"设计完成，请 Review"


Phase 3: 开发调整（在 elfiee-mvp-ui）
────────────────────────────────────
[zy/sy 本地操作]
1. git checkout feat/ui-{feature}-rh
2. git checkout -b feat/ui-{feature}-dev
3. 调整代码：
   - 修复 TypeScript 错误
   - 优化组件结构
   - 添加 PropTypes
   - 统一命名规范
4. git commit & push
      ↓
[sy/zy 创建 PR]
feat/ui-{feature}-dev → main
      ↓
[sy/zy Review PR]
- 检查代码质量
- 测试功能
- 批准或要求修改
      ↓
[sy/zy 合并到 main]
git merge feat/ui-{feature}-dev


Phase 4: 迁移到主项目（elfiee）
──────────────────────────────
[sy/zy 执行迁移]
1. 在 elfiee 创建分支
   git checkout -b feat/ui-migration-{feature}

2. 运行迁移脚本
   pnpm run migrate-ui

3. 手动集成 Tauri
   - 替换 mock 数据为 Tauri hooks
   - 更新 bindings.ts 类型

4. 测试
   pnpm tauri dev

5. 提交 PR
   feat/ui-migration-{feature} → dev
      ↓
[sy Review 和合并]


Phase 5: 后续迭代
────────────────
[有新 UI 改动]
      ↓
回到 Phase 1，重复流程
先在 elfiee-mvp-ui 稳定 → 再迁移到 elfiee
```


## 质量保证机制

### 1. CI工具：
- GitHub Actions
- Pre-commit Hooks
- PR 模板

### 4. 代码 Review 检查清单

**zy/sy 在 Review PR 时检查**：

**基础检查**：
- [ ] 分支命名符合规范
- [ ] Commit 信息清晰
- [ ] 无 console.log 调试代码
- [ ] 无注释的废弃代码

**TypeScript**：
- [ ] 所有组件有类型定义
- [ ] Props 使用 interface 声明
- [ ] 无 `any` 类型（除非必要）
- [ ] 类型从 `@/types` 正确导入

**React**：
- [ ] 组件使用函数式组件
- [ ] useState/useEffect 使用正确
- [ ] 无不必要的 re-render
- [ ] Key prop 在列表中使用

**样式**：
- [ ] 使用 TailwindCSS 类名
- [ ] 无内联样式（除非动态计算）
- [ ] 颜色使用主题变量
- [ ] 响应式断点正确

**性能**：
- [ ] 大列表使用虚拟滚动（如适用）
- [ ] 图片使用懒加载
- [ ] 复杂计算使用 useMemo
- [ ] 回调函数使用 useCallback

---

## 检查清单

### rh 设计完成时

- [ ] 在 Lovable Preview 中测试所有交互
- [ ] 确认动画效果流畅
- [ ] 检查响应式布局
- [ ] 确认颜色使用主题变量
- [ ] 在 GitHub Issue 中通知 zy/sy

### zy/sy 开发调整完成时

- [ ] 本地运行 `pnpm run dev` 无错误
- [ ] TypeScript 编译通过（`pnpm run build`）
- [ ] 运行 Lint（`pnpm run lint`）
- [ ] 代码已格式化（`pnpm run format`）
- [ ] 创建 PR 并填写完整描述

### sy/zy Review PR 时

- [ ] 检出分支并本地测试
- [ ] 验证所有交互功能
- [ ] 检查代码规范
- [ ] 确认设计符合要求（必要时 @rh 确认）
- [ ] 批准或要求修改

### sy/zy 迁移到 elfiee 时

- [ ] `elfiee-mvp-ui` 的 `main` 分支已稳定
- [ ] 运行迁移脚本
- [ ] 替换 Mock 数据为 Tauri hooks
- [ ] 更新类型导入（从 `@/bindings`）
- [ ] 运行 `pnpm tauri dev` 测试
- [ ] 所有功能正常工作
- [ ] 创建 PR 并合并到 `dev`

---
