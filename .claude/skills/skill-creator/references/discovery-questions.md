# Skill Discovery Questionnaire

This guide provides a structured approach to gather all information needed to create an effective skill through interactive Q&A.

## How to Use This Guide

When a user wants to create a new skill:
1. Start with **Phase 1** questions to understand the core purpose
2. Progress through phases based on user responses
3. Use the **Material Checklist** to identify what the user needs to provide
4. Summarize findings before proceeding to skill creation

**Important**: Ask 1-3 questions at a time. Wait for answers before asking more. Adapt follow-up questions based on responses.

---

## Phase 1: Core Purpose (Required)

Start here. These questions establish the skill's foundation.

| Question | Purpose |
|----------|---------|
| What problem does this skill solve? What task should it help accomplish? | Defines the core value |
| Give 2-3 examples of what a user might say to trigger this skill | Establishes trigger patterns |
| What should the skill produce as output? (code, document, file, action, etc.) | Clarifies expected deliverables |

**Example dialogue:**
```
Claude: "What problem does this skill solve?"
User: "I want to quickly generate API documentation from my code"
Claude: "Got it. Can you give 2-3 examples of what you might say to trigger this?"
User: "Generate docs for my API", "Document this endpoint", "Create API reference"
```

---

## Phase 2: Workflow Analysis (Required)

After understanding the purpose, explore the workflow.

| Question | Purpose |
|----------|---------|
| Walk me through how you would do this task manually, step by step | Maps the workflow |
| Are there decision points where different approaches apply? | Identifies branching logic |
| What's the most error-prone or tedious part of this process? | Highlights automation opportunities |
| How do you know when the task is done correctly? | Defines success criteria |

---

## Phase 3: Input & Context (Required)

Understand what the skill needs to work with.

| Question | Purpose |
|----------|---------|
| What inputs does this skill need? (files, data, user info, etc.) | Identifies required inputs |
| Does this skill need access to external services or APIs? | Identifies integrations |
| Is there domain-specific knowledge or terminology involved? | Surfaces reference material needs |
| Are there company-specific conventions or standards to follow? | Identifies custom requirements |

---

## Phase 4: Resource Planning (Based on Phases 1-3)

Based on earlier answers, determine what resources the skill needs.

### Scripts Needed?
Ask if the workflow involves:
- Repetitive code that would be rewritten each time
- Operations requiring deterministic reliability
- File format conversions or transformations
- API calls with specific patterns

**Question**: "You mentioned [X]. Do you have existing scripts for this, or should we create one?"

### References Needed?
Ask if the skill requires:
- Database schemas or data structures
- API documentation
- Domain knowledge (industry terms, formulas, rules)
- Company policies or guidelines
- Style guides or templates

**Question**: "You mentioned [domain knowledge]. Do you have documentation for this? Can you share it or describe the key points?"

### Assets Needed?
Ask if the skill produces:
- Documents from templates
- Code from boilerplate
- Outputs with brand elements (logos, fonts)
- Structured files (reports, presentations)

**Question**: "For the [output type], do you have existing templates or examples I should follow?"

---

## Phase 5: Boundaries & Edge Cases

Define what the skill should NOT do.

| Question | Purpose |
|----------|---------|
| What should this skill NOT attempt to do? | Sets clear boundaries |
| Are there related tasks that should be separate skills? | Prevents scope creep |
| What happens if required input is missing or invalid? | Defines error handling |

---

## Material Checklist

After completing the questionnaire, use this checklist to confirm what materials are needed.

### User Must Provide:
- [ ] **Example inputs** - Sample files or data the skill will process
- [ ] **Expected outputs** - Examples of desired results
- [ ] **Reference docs** - Any documentation the skill should follow
- [ ] **Templates** - If outputs should match a specific format
- [ ] **Credentials/Config** - If external services are involved (document, don't include in skill)

### Claude Will Create:
- [ ] **SKILL.md** - Main instruction file
- [ ] **Scripts** - Automation code identified in Phase 4
- [ ] **Reference summaries** - Condensed domain knowledge for context

### Information Summary Template

Before proceeding to create the skill, summarize:

```markdown
## Skill Summary: [skill-name]

### Purpose
[One sentence describing what the skill does]

### Trigger Examples
- "[example 1]"
- "[example 2]"
- "[example 3]"

### Workflow
1. [Step 1]
2. [Step 2]
3. [Step 3]

### Inputs Required
- [Input 1]: [description]
- [Input 2]: [description]

### Outputs Produced
- [Output description]

### Resources to Create
- scripts/: [list or "none"]
- references/: [list or "none"]
- assets/: [list or "none"]

### Materials Needed from User
- [ ] [Item 1]
- [ ] [Item 2]

### Boundaries
- Will NOT: [boundary 1]
- Will NOT: [boundary 2]
```

---

## Quick Reference: Question Selection

| If user says... | Ask about... |
|----------------|--------------|
| "automate X" | Workflow steps, repetitive parts, scripts |
| "generate X" | Output format, templates, examples |
| "help with X domain" | Domain knowledge, terminology, references |
| "integrate with X" | API docs, authentication, error handling |
| "follow our standards" | Company docs, style guides, policies |

---

## Anti-Patterns to Avoid

- **Don't ask all questions at once** - Overwhelms the user
- **Don't assume** - Verify understanding with examples
- **Don't skip workflow analysis** - Even "simple" tasks have hidden complexity
- **Don't forget boundaries** - Undefined scope leads to poor skills
