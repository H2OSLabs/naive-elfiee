# Elfiee 团队协作快速开始指南

**目标受众**: sy, zy, rh
**预计完成时间**: 30 分钟
**更新日期**: 2025-12-03

---

## 📋 前置检查清单

在开始前，确保以下条件满足：

### 所有人

- [ ] GitHub 账户已加入 `H2OSLabs` 组织
- [ ] 已安装 Git
- [ ] 熟悉基本 Git 命令（clone, checkout, commit, push, pull）

### rh（设计师）

- [ ] Lovable Pro 账户已激活
- [ ] Lovable 已绑定 GitHub 账户
- [ ] 已导入 `elfiee-mvp-ui` 项目到 Lovable

### zy（开发者）

- [ ] Node.js 18+ 已安装
- [ ] pnpm 已安装（`npm install -g pnpm`）
- [ ] VS Code 已安装
- [ ] VS Code 扩展：ESLint, Prettier, Tailwind CSS IntelliSense

### sy（Owner & Tech Lead）

- [ ] 所有 zy 的环境
- [ ] Rust + Cargo 已安装
- [ ] Tauri CLI 已安装（`cargo install tauri-cli`）
- [ ] 对 `elfiee` 和 `elfiee-mvp-ui` 仓库有 Admin 权限

---

## 🚀 立即开始：第一个功能迭代

我们将通过一个完整的示例，演示如何协作完成一个 UI 改进。

### 示例任务：优化 BlockHUD 的悬停效果

---

## 步骤 1：sy 发起讨论（5 分钟）

**sy 操作**：

1. 访问 https://github.com/H2OSLabs/elfiee-mvp-ui/issues
2. 点击 **New Issue**
3. 填写 Issue：

```markdown
标题：[Feature] 优化 BlockHUD 悬停效果

## 背景
当前 BlockHUD 在鼠标悬停时显示，但视觉效果不够明显，用户容易忽略。

## 提议方案
1. 背景色改为半透明黑色（rgba(0, 0, 0, 0.85)）
2. 添加淡入动画（200ms）
3. 当 Block 被选中时，HUD 保持可见

## 讨论点
@rh 你觉得这个方案如何？有更好的设计想法吗？
@zy 技术实现上有问题吗？

## 验收标准
- [ ] 悬停时 HUD 明显可见
- [ ] 动画流畅自然
- [ ] 选中状态下 HUD 保持显示
- [ ] 无性能问题
```

4. 添加标签：`enhancement`, `ui-design`
5. 分配给 `@rh`

---

## 步骤 2：rh 在 Lovable 中设计（10 分钟）

**rh 操作**：

### 2.1 创建分支

1. 登录 Lovable（https://lovable.dev）
2. 打开项目 `elfiee-mvp-ui`
3. 点击左下角当前分支名（应该是 `main`）
4. 点击 **Create new branch**
5. 输入分支名：`feat/block-hud-hover`（基于 `main`）
6. 点击 **Create**

### 2.2 提交设计 Prompt

在 Lovable 的聊天框中输入：

```
修改 BlockHUD 组件（src/components/BlockHUD.tsx）：

1. 背景色改为 rgba(0, 0, 0, 0.85)
2. 添加淡入动画：
   - 初始状态：opacity: 0
   - 悬停后：opacity: 1
   - 过渡时间：200ms
   - 缓动函数：ease-in-out
3. 当 isSelected 为 true 时，HUD 始终可见（不需要悬停）
4. 图标颜色改为白色

请保持其他功能不变。
```

### 2.3 验证效果

1. 等待 Lovable 生成代码（通常 10-30 秒）
2. 点击 **Preview** 查看效果
3. 测试：
   - 鼠标悬停在 Block 上，HUD 是否淡入？
   - 动画是否流畅？
   - 选中 Block 后，HUD 是否保持可见？

### 2.4 微调（如果需要）

如果效果不满意，继续输入 Prompt：

```
淡入动画太慢了，改为 150ms
```

或

```
背景色太暗了，改为 rgba(0, 0, 0, 0.7)
```

### 2.5 完成后通知

1. 确认满意后，Lovable 会自动推送代码到 GitHub
2. 回到 GitHub Issue，评论：

```markdown
@zy @sy 设计已完成！

分支：feat/block-hud-hover
Lovable Preview: [粘贴 Lovable 的 share 链接，如果有]

主要改动：
- 半透明黑色背景
- 200ms 淡入动画
- 选中状态保持可见

请 Review 代码并进行必要调整。
```

---

## 步骤 3：zy 本地调整代码（10 分钟）

**zy 操作**：

### 3.1 克隆仓库（如果还没有）

```bash
# 只需第一次执行
cd ~/projects
git clone git@github.com:H2OSLabs/elfiee-mvp-ui.git
cd elfiee-mvp-ui
pnpm install
```

### 3.2 检出 rh 的分支

```bash
cd ~/projects/elfiee-mvp-ui

# 拉取最新代码
git fetch origin

# 切换到 rh 的分支
git checkout feat/block-hud-hover
git pull origin feat/block-hud-hover

# 安装依赖（如果 rh 添加了新库）
pnpm install
```

### 3.3 本地运行查看效果

```bash
pnpm run dev
```

浏览器会自动打开 `http://localhost:5173`，测试 BlockHUD 的效果。

### 3.4 调整代码

打开 `src/components/BlockHUD.tsx`，可能需要调整：

**1. 添加 TypeScript 类型**（如果 Lovable 没有生成）

```typescript
// 在文件顶部添加
interface BlockHUDProps {
  block: Block
  isSelected: boolean
  onOpenDrawer: (tab: "history" | "access" | "agent") => void
  onRun?: () => void
}

export const BlockHUD = ({ block, isSelected, onOpenDrawer, onRun }: BlockHUDProps) => {
  // 组件代码
}
```

**2. 修复动画实现**（如果 Lovable 使用了不推荐的方式）

```typescript
// 如果 Lovable 使用了内联样式，改为 Tailwind 类
// ❌ Lovable 可能生成的
<div style={{ opacity: isHovered ? 1 : 0, transition: '200ms' }}>

// ✅ 改为
<div className={`transition-opacity duration-200 ${isHovered || isSelected ? 'opacity-100' : 'opacity-0'}`}>
```

**3. 运行代码检查**

```bash
# 格式化代码
pnpm run format

# 检查 Lint
pnpm run lint

# 构建测试
pnpm run build
```

### 3.5 提交代码

```bash
# 查看修改
git status
git diff

# 提交（直接在 rh 的分支上）
git add src/components/BlockHUD.tsx

git commit -m "refactor: optimize BlockHUD hover effect

- Add TypeScript types for props
- Replace inline styles with Tailwind classes
- Ensure animation works with both hover and selected states

Co-authored-by: rh <rh@example.com>"

# 推送
git push origin feat/block-hud-hover
```

### 3.6 创建 Pull Request

1. 访问 https://github.com/H2OSLabs/elfiee-mvp-ui
2. GitHub 会显示 "feat/block-hud-hover had recent pushes"
3. 点击 **Compare & pull request**
4. 填写 PR：

```markdown
## 改动描述
优化 BlockHUD 悬停效果，包含设计（@rh）和代码调整（@zy）。

关联 Issue: #1

## 主要修改
- ✅ 半透明黑色背景（@rh）
- ✅ 200ms 淡入动画（@rh）
- ✅ 选中状态保持可见（@rh）
- ✅ 添加 TypeScript 类型（@zy）
- ✅ 使用 Tailwind 类替换内联样式（@zy）

## 测试清单
- [x] 本地运行正常
- [x] 悬停动画流畅
- [x] 选中状态 HUD 保持可见
- [x] TypeScript 编译无错误
- [x] Lint 检查通过

## 截图
[可选：附上 Before/After 截图]

## Review 清单
- [ ] @sy 代码 Review
- [ ] @rh 设计验收
```

5. 点击 **Create pull request**

---

## 步骤 4：sy Review 和合并（5 分钟）

**sy 操作**：

### 4.1 本地检出 PR

```bash
cd ~/projects/elfiee-mvp-ui
git fetch origin
git checkout feat/block-hud-hover
git pull origin feat/block-hud-hover

# 运行测试
pnpm run dev
```

### 4.2 验证功能

浏览器中测试：
- [ ] BlockHUD 悬停效果是否符合预期
- [ ] 动画是否流畅
- [ ] 选中状态是否正确
- [ ] 无控制台错误

### 4.3 检查代码质量

```bash
# TypeScript 检查
pnpm run build
# 预期：无错误

# Lint 检查
pnpm run lint
# 预期：无警告
```

### 4.4 批准并合并

1. 访问 PR 页面
2. 点击 **Files changed** 查看代码差异
3. 如果一切正常，点击 **Review changes** → **Approve** → **Submit review**
4. 点击 **Squash and merge**
5. 确认合并信息：

```
feat: optimize BlockHUD hover effect (#1)

- Add semi-transparent black background
- Add 200ms fade-in animation
- Keep HUD visible when block is selected
- Improve TypeScript types

Co-authored-by: rh <rh@example.com>
Co-authored-by: zy <zy@example.com>
```

6. 点击 **Confirm squash and merge**
7. 删除分支（可选）：点击 **Delete branch**

### 4.5 关闭 Issue

1. 访问原始 Issue
2. 评论：

```markdown
✅ 已完成并合并到 main 分支（PR #1）

测试结果：
- ✅ 悬停动画流畅
- ✅ 选中状态正常
- ✅ 无性能问题

下一步：等待积累更多 UI 改动后，统一迁移到 elfiee 主项目。
```

3. 点击 **Close issue**

---

## 步骤 5：sy 迁移到 elfiee 主项目（当积累足够改动时）

**注意**：不要每个小改动都迁移，建议积累 5-10 个 PR 后再迁移。

当准备迁移时，sy 执行以下步骤：

### 5.1 准备工作

```bash
# 确保两个仓库都是最新的
cd ~/projects/elfiee-mvp-ui
git checkout main
git pull origin main

cd ~/projects/elfiee
git checkout dev
git pull origin dev

# 创建迁移分支
git checkout -b feat/ui-migration
```

### 5.2 手动 Copy-Paste 组件

**方法 1：使用两个 VS Code 窗口**

```bash
# 终端 1
cd ~/projects/elfiee-mvp-ui
code .

# 终端 2
cd ~/projects/elfiee
code .
```

然后逐个复制文件：
1. 在 `elfiee-mvp-ui` 中打开组件文件
2. 全选代码（Cmd/Ctrl + A）
3. 复制（Cmd/Ctrl + C）
4. 切换到 `elfiee` 窗口
5. 打开或创建同名文件
6. 粘贴（Cmd/Ctrl + V）
7. 保存

**方法 2：使用命令行复制**

```bash
cd ~/projects/elfiee

# 复制整个 components 目录
cp -r ../elfiee-mvp-ui/src/components/* ./src/components/

# 复制 pages
cp -r ../elfiee-mvp-ui/src/pages/* ./src/pages/

# 复制样式
cp ../elfiee-mvp-ui/src/index.css ./src/index.css

# 检查复制结果
git status
```

### 5.3 生成 Tauri bindings

```bash
cd ~/projects/elfiee/src-tauri

# 构建项目（自动生成 bindings.ts）
cargo build

# 返回项目根目录
cd ..

# 检查 bindings.ts 是否生成
ls -la src/bindings.ts
```

### 5.4 更新组件以使用 Tauri hooks

编辑 `src/pages/Index.tsx`（或 `src/App.tsx`）：

```typescript
// ❌ 删除 Mock 数据导入
// import { mockBlocks, mockEvents } from '@/data/mockElfieeData'

// ✅ 导入 Tauri hooks（如果还没有，需要先创建这些 hooks）
import { useElfieeBlocks, useElfieeEvents } from '@/hooks'

function App() {
  const currentFileId = "doc1"

  // ✅ 使用 Tauri hooks
  const { blocks, loading } = useElfieeBlocks(currentFileId)
  const { events } = useElfieeEvents(currentFileId)

  if (loading) {
    return <div>Loading...</div>
  }

  // UI 部分保持不变
  return (
    <div className="h-screen">
      <DocumentView blocks={blocks} events={events} />
    </div>
  )
}
```

### 5.5 更新类型导入

```bash
# 查找所有从 @/types 导入的地方
grep -r "from '@/types'" src/

# 手动将后端类型改为从 @/bindings 导入
# 例如：
# 改前：import { Block, Event } from '@/types'
# 改后：import { Block, Event } from '@/bindings'
```

或使用查找替换：

```bash
find src -name "*.tsx" -type f -exec sed -i \
  "s/import { \(Block\|Event\|Editor\|Capability\) } from '@\/types'/import { \1 } from '@\/bindings'/g" \
  {} \;
```

### 5.6 测试集成

```bash
cd ~/projects/elfiee

# 运行 Tauri 开发服务器
pnpm tauri dev
```

测试：
- [ ] 应用启动成功
- [ ] Blocks 从 Tauri 后端加载
- [ ] BlockHUD 悬停效果正常
- [ ] 无控制台错误

### 5.7 提交并创建 PR

```bash
cd ~/projects/elfiee

git add src/
git commit -m "feat: migrate UI improvements from elfiee-mvp-ui

Migrated components:
- BlockHUD with optimized hover effect
- [其他改动列表]

Changes:
- Replaced mock data with Tauri hooks
- Updated type imports from bindings.ts
- Removed mockElfieeData dependencies

Co-authored-by: rh <rh@example.com>
Co-authored-by: zy <zy@example.com>"

git push origin feat/ui-migration
```

创建 PR：`feat/ui-migration` → `dev`，等待 Review 后合并。

---

## 🎯 总结：完整工作流程

```
1. sy 创建 Issue
   ↓
2. rh 在 Lovable 设计（feat/xxx 分支）
   ↓
3. zy 本地调整代码（同一分支）
   ↓
4. zy 创建 PR → main
   ↓
5. sy Review & 合并
   ↓
6. 积累多个 PR 后，sy 迁移到 elfiee
   ↓
7. sy 提交 PR → dev
```

---

## 🛠️ 常用命令速查

### rh（Lovable）

```
# 无需命令行，全部在 Lovable Web 界面操作
```

### zy（开发者）

```bash
# 第一次设置
cd ~/projects
git clone git@github.com:H2OSLabs/elfiee-mvp-ui.git
cd elfiee-mvp-ui
pnpm install

# 每次开始新功能
git fetch origin
git checkout feat/xxx
git pull origin feat/xxx
pnpm run dev

# 提交代码
git add .
git commit -m "refactor: xxx"
git push origin feat/xxx

# 代码检查
pnpm run lint
pnpm run build
```

### sy（Owner）

```bash
# Review PR
cd ~/projects/elfiee-mvp-ui
git checkout feat/xxx
pnpm run dev

# 迁移到 elfiee
cd ~/projects/elfiee
git checkout -b feat/ui-migration
# ... 手动复制文件 ...
cd src-tauri && cargo build && cd ..
pnpm tauri dev
git commit -m "feat: migrate UI"
git push origin feat/ui-migration
```

---

## ❓ 常见问题

### Q: rh 在 Lovable 中如何查看现有代码？

**A**:
1. 在 Lovable 中点击左侧的 **Files** 图标
2. 浏览文件树
3. 点击文件名查看代码

### Q: zy 如何知道 rh 已经完成设计？

**A**: rh 会在 GitHub Issue 中 @ 你，并说明分支名称。

### Q: 如果 zy 调整代码后 rh 想继续修改怎么办？

**A**:
1. zy 先推送代码到 GitHub
2. Lovable 会自动拉取更新
3. rh 可以继续在 Lovable 中修改

### Q: 什么时候应该迁移到 elfiee？

**A**:
- 积累了 5-10 个 PR
- 或者完成了一个完整的功能模块
- 或者 `elfiee-mvp-ui` 的 UI 已经稳定

---

## 📚 相关文档

- 完整协作流程：`TEAM_COLLABORATION_WORKFLOW.md`
- Lovable 编辑规范：`LOVABLE_EDITING_GUIDELINES.md`
- Lovable Pro 设置指南：`LOVABLE_PRO_SETUP_GUIDE.md`

---

**准备好了吗？立即开始第一个迭代！🚀**
