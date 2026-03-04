

> 本文档用于定义 Elfiee Phase 2 的核心 Dogfooding 实验。 这一阶段的 Dogfooding 不是为了证明“我们能不能把东西记下来”，而是为了验证： **这些记录是否真的在第二次被使用，并带来了可度量的学习收益**。
> 
> 因此，本实验的唯一北极星问题是： **在同一task type上，第二次使用 Elfiee 时，完成任务的效率与准确度，是否相较第一次出现系统性提升？**



## 1   实验目标

验证：

> 历史记录是否在 Second-use 中被复用，并对结果产生了可测量的正向影响。

注意：在 Phase 2，**记录（Record）不是验证目标，而是前置条件**。


“正向影响”，可以被拆解为三个层面的结果：

1. **效率影响**  
    Second-use 相比 First-use：
    
    - 更快进入有效执行状态
        
    - 更少中途澄清与反复确认
        
    - 更少返工 / 回退
        
2. **准确度影响**  
    Second-use 相比 First-use：
    
    - 更少关键约束遗漏
        
    - 更少方向性错误
        
    - 更少事后纠偏解释
        
3. **学习**
	Second-use 过程中：
	
	- 引用了 First-use 所产生的记录
		
	- 修改了 First-use 所产生的记录

> 若记录存在，但未在 Second-use 中带来上述任一类改进，则在 Phase 2 视角下我们的设计是不够合理的。



### 1.1   Phase 2 成功判定

> **在同一 Task Type 下，Second-use 在效率或准确度指标上，相较 First-use 出现稳定、可解释的改善。**

说明：

- 不要求同时提升效率与准确度
    
- 只要任一维度出现明确改善，即视为学习发生
    


Phase 2 可以结束的最低条件：

- ≥1 组完整的 Task Pair（First / Second）
    
- 在效率或准确度指标中至少观察到 1 项明确改善
    
- 改善结果可被历史记录引用行为合理解释
    

> 若不满足以上条件，则说明： **当前记录与表达方式尚不足以支撑“可学习决策”。**

### 1.2   本实验明确不做的事情

- 不证明“Elfiee 比不用好”
    
- 不追求统计显著性
    
- 不评估功能体验是否友好
    
- 不验证 Skill 的长期稳定性
    

Phase 2 是一个 **学习是否发生的自举验证**，而非产品成熟度评估。


### 1.3   预计实验能得出的其他观点


- 哪些 Task Type 具备明显的 Second-use 学习收益
    
- 哪些 Skill 在 Second-use 中真实发挥作用
    
- 哪些记录形态无法支持学习，应被淘汰或重构
    
## 2   实验场景与数据筛选

### 2.1   实验场景要求

**Unit = One Task Type × Two Rounds（First-use / Second-use）**

Phase 2 只认可如下实验结构：

- 同一 Task Type
    
- 至少执行两次
    
- Second-use 明确发生在“可访问 First-use 历史记录”的前提下


### 2.2   场景流程、指令需求

#### 环境准备（一次性）


| 阶段  | 角色  | 操作描述                            | 对应 API / 系统能力                                                       | 备注                                  |
| --- | --- | ------------------------------- | ------------------------------------------------------------------- | ----------------------------------- |
| A1  | Dev | 创建 / 打开 `project.elf`           | `core.create_project` / `core.open_project`                         | GUI 内部完成 EventStore / Projector 初始化 |
| A2  | Dev | 导入外部代码仓库                        | `directory.import(external_path)`                                   | 需记录 `external_root_path`            |
| A3  | Dev | 创建并启用 Agent Block 和 Skill Block | `agent.create(target_project_id)`<br>`agent.enable(agent_block_id)` | 自动合并 MCP 配置                         |
| A4  | Dev | 重启 Claude Code                  | *无 API*                                                             | Claude 自动连接 MCP Server              |
#### 定义task
| 顺序   | 角色           | 用户操作                  | 系统 / API 行为                      | 产品侧关心点       |
| ---- | ------------ | --------------------- | -------------------------------- | ------------ |
| B0-1 | PM           | 创建 Task Block         | `core.create(block_type="task")` |              |
| B0-2 | PM           | 填写需求、约束、验收标准          | `task.write(...)`                | 约束是否结构化      |
| B0-3 | PM           | 填写 Task Type          | `task.write({task_type})`        | **实验分组唯一依据** |
| B0-4 | PM/Dev/Agent | （可选）引用历史 Task / Skill | `core.read(...)`                 | 若无历史，则无反应    |


#### 执行任务

| 顺序   | 角色          | 用户操作                 | 系统 / API 行为                            | 产品侧关心点            |
| ---- | ----------- | -------------------- | -------------------------------------- | ----------------- |
| B1-1 | Dev         | 在 Claude 中开始做这个 Task | `core.read(task_id)`                   | Task → Session 绑定 |
| B1-2 | Dev / Agent | 写 / 改代码              | `code.write(...)`                      | 文件快照是否完整          |
| B1-3 | Dev / Agent | 建立 Task → Code 实现关系  | `core.link(relation="implement")`      | 是否遗漏 implement    |
| B1-4 | Dev         | 与 Claude 反复对话        | Session JSONL 同步                       | Clarification 数据源 |
| B1-5 | Dev         | 运行测试 / 编译            | `terminal.run(command)`                | **FPY 指标采集**      |
| B1-6 | 系统          | 记录验证结果               | Event 记录 `test_result: success / fail` | 是否一次通过（FPY）       |


| 阶段  | 角色       | 行为描述           | 系统 / API 行为                        | 产品侧关心点                |
| --- | -------- | -------------- | ---------------------------------- | --------------------- |
| 执行中 | Dev / PM | 确认 AI 生成的阶段性总结 | `summary.confirm` / `summary.edit` | **Summary 采纳率（编辑距离）** |


#### commit

| 顺序   | 角色     | 用户操作                 | 系统 / API 行为                        | 产品侧关心点      |
| ---- | ------ | -------------------- | ---------------------------------- | ----------- |
| B3-1 | Dev    | 在 Claude 中发起提交       | `task.commit(task_id, push:false)` |             |
| B3-2 | 系统     | 导出文件并自动 Git Commit   | `directory.export` → `git commit`  | commit 是否稳定 |
| B3-3 | 系统     | Task 状态 → Committed  | 内部状态更新                             | TTE 计算边界    |
| B3-4 | PM | 验收测试：拉取代码并运行验收脚本 | `directory.export`                 | 确定最终质量      |

#### Archive

| 顺序   | 角色  | 用户操作               | 系统 / API 行为                       | 产品侧关心点   |
| ---- | --- | ------------------ | --------------------------------- | -------- |
| B4-1 | Dev | 归档任务               | `task.archive(task_id)`           |          |
| B4-2 | 系统  | 生成归档 Markdown      | `.elf/Archives/{date}-{title}.md` | 是否包含全部关联 |
| B4-3 | 系统  | Task 状态 → Archived | 内部状态更新                            | 复用入口     |


#### 实验视角下事后区分 first / second









### 2.3   Task Type 的约束条件

一个 Task Type 是否合格，取决于是否满足：

- **结构相似性**：
    
    - 规则 / 阈值 / 流程 / 边界判断形态相似
        
- **存在犯错空间**：
    
    - First-use 有合理概率出现遗漏或误判
        

示例：

- 带复杂业务规则的配置修改
    
- 涉及边界条件的 API 逻辑实现
    
- 含多分支判断的流程型改动
    

## 3   指标体系


### 3.1   效率指标（Efficiency Metrics）

| **指标**                         | **定义与计算公式**                                                                                | **数据采集要求（埋点与统计逻辑）**                                                                                                                                        |
| ------------------------------ | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Time to First Commit (TFC)** | 从任务创建到首次有效代码变更的冷启动耗时。<br><br>公式：$T(\text{First Artifact Event}) - T(\text{Task Creation})$ | **1. 系统需记录**：`task.create` 事件的时间戳；<br><br>**2. 系统需筛选**：该 `task_id` 下关联的首个 `code.write` 或 `git.stage`事件的时间戳。                                                |
| **Clarification Count**        | 任务执行过程中，针对需求约束或业务规则的澄清轮次。                                                                  | **1. 定义“澄清事件”**：由 `actor=human` 发起，且通过文本分析后打标为 `type=clarify` 的对话 Event（询问需求含义、确认规则或边界条件、要求补充缺失信息）；<br><br>**2. 系统需统计**：该 `task_id` 关联的所有符合上述条件的 Event 总数。 |
| **Rework Count**               | 方案被推翻并执行回退或大规模重构的次数。                                                                       | **1. 采集动作**：系统需捕捉 `core.undo`、`git.revert` 、 `task.status` 从 `InProgress` 回跳至 `Pending` 的行为；<br><br>**2. 统计逻辑**：记录被标记为 `action_type=rollback` 的事件频率。       |

### 3.2   准确度指标（Accuracy Metrics）

| **指标**                      | **定义**                         | **数据采集要求（埋点与统计逻辑）**                                                                                                          |
| --------------------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| **Constraint Miss Count**   | 在任务完成（Committed）后，复盘发现的遗漏约束条数。 | **1. 人工标注**：对比 task 与最终产出，记录最终产出遗漏的约束项数量。                                                                                    |
| **Directional Error Count** | 判定该次任务是否存在“整体方向性错误”。           | **1. 统计逻辑**：在 `task.archive` 时，由 PM 根据任务经历了“推倒重来”的次数。                                                                        |
| **Post-hoc Fix Count**      | 主要开发完成后，为了纠偏而追加的补丁或解释数量。       | **1. 系统记录**：识别 `task.status=Committed` 之后的首个 `code.write` 事件；<br><br>**2. 筛选逻辑**：统计任务标记为“基本完成”后，到“正式归档”之间产生的 `type=fix` 事件数。 |

### 3.3   学习行为指标（Learning Signals）

| **指标**                      | **定义**                      | **数据采集要求（埋点与统计逻辑）**                                                                                                                      |
| --------------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| **History Reference Count** | Second-use 中主动调用或引用历史资产的频次。 | **1. 埋点要求**：系统需记录 `mcp.call` 或 `core.read` 行为；<br><br><br>**2. 统计数据**：筛选 `artifact_ref` 指向 `previous_task_id` 或旧版 `skill_id` 的 Event 计数。 |
| **Skill Reuse / Update**    | 对既有 Skill 的复用率或迭代率。         | **1. 采集动作**：记录 `skill.invoke`（复用）与 `skill.write`（更新）事件；<br><br>**2. 归因逻辑**：系统需自动关联该 Skill 的来源，判断其是否生成于 First-use 阶段。                     |

## 4   数据采集与计算方法



### 4.1   Event Schema

每一条 Event 表示一次**可定位、可回放的行为或决策节点**，必须包含以下字段：

|              |                                    |
| ------------ | ---------------------------------- |
| 字段           | 定义                                 |
| task_id      | 该事件所属的 Task 唯一标识                   |
| task_type    | 该 Task 对应的任务类型标签                   |
| task_round   | First-use 或 Second-use             |
| actor        | 行为执行者（human / agent）               |
| artifact_ref | 被操作或引用的对象（skill / file / commit 等） |
| timestamp    | 事件发生时间                             |

## 5   归因分析框架

对每一组 Task Pair（First / Second），逐项回答以下问题：

### 5.1   为什么 Second-use 更高效？

参考指标：

- TTE Delta
    
- Clarification Count Delta
    
- History Reference Count
    
- Skill Reuse Count
    

分析重点：

- 是否因为直接复用了历史约束或 Skill，从而跳过澄清阶段
    

### 5.2   为什么 Second-use 更准确？

参考指标：

- Constraint Miss Count Delta
    
- Directional Error Count Delta
    
- Skill Update Count
    

分析重点：

- 哪些历史决策帮助避免了首次犯过的错误
    

---

### 5.3   如果没有这些记录，会发生什么？

参考指标：

- First-use 与 Second-use 的全部 Delta 对比
    
- Learning Signals 是否为 0 的对照情况
    

分析重点：

- 哪些改进可以合理归因于历史记录，而非偶然或熟练度提升
