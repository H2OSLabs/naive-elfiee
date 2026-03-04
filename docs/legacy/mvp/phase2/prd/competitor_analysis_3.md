## 1   研究问题

1. 现有工作流为什么没有elfiee这么AI-friendly？（为什么是我们来做elfiee，为啥不用是Google）

	* 不再关注竞品“能不能做这个功能”（因为功能总能加），而是关注竞品的工作流本质是否导致它们“在结构上必然无法实现决策资产化”。

	* 我们要证明：Elfiee 要解决的问题，是这些工具因其底层逻辑（管理流、文档流、单次生成流）而**注定无法解决**的。
2. 其他产品的skills
3. 其他产品的内外双循环
4. 其他问题（backlog）
	1. terminal 验证是否真的不可或缺？
		1. 没有开发，设计师和产品无法自我验证？
	2. 有些失败的决策是被故意遗忘的（组织/社会因素）
	3. AI可读取的数据范围：看到文档之间的因果链 vs. 看到单个文档 vs. 看到很多文档
	4. 跨工具是否真的是问题
		1. 没有一个工具能持续陪跑、对这条链路负责

## 2   Elfiee 工作流梳理

### 1.1   Phase 0｜创建项目

**用户行为**

1. 在 Elfiee 中创建 Project
    
2. 在 **Linked Repository** 挂载本地 / 外部 repo（repo1）
    
3. 在 **Directory** 中创建：
    - Agent Skill 文档（block 化，权威版本在 Elfiee）
        
    - Task 文档（需求 + 验证标准，block 化）
        

**系统资产**

- `TaskBlock@v0`
    
- `SkillBlock@v0`
    
- Project ↔ repo1 的绑定关系

### 1.2   Phase 1｜Skill 导出

**用户行为**

4. 在 Elfiee 中点击 **Export Skills to Repo**
    
5. Skill 以 `.elfiee/skills/*.md` 写入 repo1
    

**系统资产**

- `SkillExportEvent`
    
    - skill_id
        
    - revision_id
        
    - target_repo
        
    - commit / fs state

### 1.3   Phase 2｜执行

**用户行为**

6. 在 Elfiee 的 **Terminal** 中：
    
    - `cd repo1`
        
    - 启动 `claude`
        
7. 明确指令 Claude：
    
    > “Read .elfiee/skills and implement Task X”
    
8. 与 Claude 多轮交互，允许失败、回滚、重试
    

**系统自动行为**

- 自动开启 **Execution Session**
    
- 捕获：
    
    - **Log2**：Terminal IO 全量（对话、失败、反思）
        
    - **Log1**：IDE / FS 层逐次文件编辑事件流（非 commit）
        

**系统资产**

- `ExecutionSession`
    
- `ChatEvent[]`
    
- `FileEditEvent[]`

### 1.4   Phase 3｜验证

**用户行为**

1. 在 Elfiee Terminal 中运行测试：
    
    - `npm test` / `cargo test` 等
        

**系统自动行为**

- 捕获测试运行
    
- 生成 **Verification Event**
    

**系统资产**

- `VerificationEvent`
    
    - command
        
    - env (repo / commit-ish)
        
    - result (pass / fail)
        
    - output_ref
        
    - linked_session_id
        
### 1.5   Phase 4｜资产化

**用户行为**

2. 点击 Task 的 **Finish / Stop Session**
    

**系统自动行为（全自动）**

- 基于 **逻辑时间**：
    
    - 绑定 ExecutionSession
        
    - 绑定 log1（文件编辑事件区间）
        
    - 绑定 log2（对话区间）
        
    - 绑定 VerificationEvent
        
- 可选：生成初版 Summary（人审 or AI）
    

**系统资产**

- `DecisionBundle`
    
    - task_block@revision
        
    - skill_block@revision
        
    - execution_session_id
        
    - file_edit_range
        
    - chat_range
        
    - verification_event


## 3   问题1：现有工作流哪里不够AI-friendly？

| **工作流**           | **Record：能不能记下高价值的决策上下文？**                                                                                        | **Learn：AI 能不能消费这些记录来干活？**                                              | **核心缺陷 **                                                               |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **国内大厂工作流**   | 不能<br><br>没有完整的决策上下文，只有流程（例如审批）上下文<br><br>记录分散在 PRD / 评论 / MR 中，被否定方案、失败决策默认被遗忘 | AI 无法消费 PRD + 代码 + 变更原因的完整链路<br><br>系统不区分“谁的判断反复被验证正确” | relation的目的是管理、免责和风控，而不是积累智慧。                          |
| **Google 生态**      | 记录，但不区分好坏。comments不可引用。<br><br>总结基于文本，不基于行为。                                                          | 1. comments 无法饮用<br>2. 无法基于项目逻辑写作，只能基于字面逻辑写作                 | 解释一切，无论实际行为是什么。就像notebook LM不管提供的书是鸡汤还是真经典。 |
| **Vibe Coding**      | chat session 清空就没有了。需要手动更新到文档里。                                                                                 | AI 只读最终代码，读不到被否定方案。                                                   | 一次性生产力。                                                              |
| **Elfiee (Phase 2)** |                                                                                                                                   |                                                                                       |                                                                             |
     

## 5   问题2：其他产品的“skills”（进行中）

## 6   问题3：其他产品的内外双循环（进行中）


| **特性**             | **Elfiee (Phase 2)**                                             | **Cursor / Windsurf**                                                                         | **Plandex / Aider**                                           | **Jira+Github**                       | **Emacs Org-mode (文学编程)**                   |
| -------------------- | ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------- | ----------------------------------------------- |
| **架构理念**         | 双循环<br><br><br>意图与实现分离，靠 Agent 桥接                  | 单循环<br><br><br>Chat 即 IDE，意图直接作用于代码                                             | 不完整的双循环<br><br><br>在内存/临时区构建意图，Apply 到文件 |                                       | 相对完整的双循环<br><br>意图与实现分离<br>      |
| **内部资产层**       | **Block & Relation**<br><br><br>结构化的决策树，包含 L1/L2 Skill | **Chat History**<br><br><br>线性的、易丢失的会话流                                            | **Plan State**<br><br><br>版本化的任务上下文                  |                                       | **Org File**<br><br><br>结构化的文本大纲        |
| **外部执行层**       | **Terminal + Repo**<br><br><br>外部工具，受Elfiee 观测           | **Built-in Editor**<br><br><br>直接修改编辑器 buffer                                          | **File System**<br><br><br>直接修改磁盘文件                   |                                       | **Source Files**<br><br><br>导出的产物          |
| **验证回填**         | **Verification Event**<br><br><br>结构化回填测试结果             | 无 / 弱<br><br><br>用户自己看 Terminal                                                        | 无<br><br><br>用户自己跑                                      |                                       | **Result Block**<br><br><br>文本回填            |
| **元学习 (Phase 5)** | **Skill Evolution**<br><br><br>显性更新 L1 Skill （系统级）      | 无<br>                                                                                        | 弱<br><br><br>用户自己存 Prompt Template                      |                                       | **强但手动**<br><br><br>用户自己改 Org 模板     |
| **适用人群**         | MVP目标是**PM + Dev**<br><br><br>关注“为什么做”和“逻辑闭环”      | **Dev**<br><br><br>关注“怎么写快点”                                                           | **Hacker / Solo Dev**<br><br><br>关注“大规模重构”             |                                       | **Geek / Researcher**<br><br><br>关注“可复现性” |
| **核心断点**         | --                                                               | 1. short vs. long contextual memory: chat session 中的知识没有资产化<br><br>2. 没有元学习环节 | 1. 没有验证环节                                               | 1. 文档和代码在两个物理隔离的数据库里 |                                                 |
| **用户真实痛点**     | 学习成本高，需要接受新范式                                       | 每次新任务都要重新想上次我是怎么跟 AI 说的来着                                                | 重构/复杂任务做完就结束，下次还是要重新设计                   |                                       | 依赖个人的复盘习惯                              |
这种市场空缺，是不是说明我们的实验很危险？其他产品可能都在一种循环上追求极致，但我们却在追求大而全？（未分析）

## 7   问题4：哪种资产化方式，能让AI更好地学习？（实验）

见 docs/mvp/phase2/experiment_plan.md
