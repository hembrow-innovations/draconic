---
id: architecture-26
type: doc
kind: architecture
title: "Task"
domain: <domain>   # domain the doc concerns, e.g. system
created_at: 2026-06-29
updated_at: 2026-06-29
---
### Task Model
**Location**: `planning/<domain>/tasks/`
**Filename**: `tasks<N>-<slug>.md`

#### Frontmatter
_Required frontmatter fields always included_
```yaml
...Required frontmatter fields
title: "task title"
description: "one sentence description"
status: "hold" | "ready" | "active" | "complete"
priority: "low" | "medium" | "high"
tags: ["list of tags"]
labels: "feature" | "bug" | "refactor" | etc
due-date: "ISO Date"

```

Status is a 4-state machine enum. Everything else a task can "be" (priority,
ready-for-agent, enhancement, bug…) rides on **tags**, not status.

#### Body / Content
```markdown
# {Title}

## Description
A clear, concise summary of what this task is about. Include the problem or opportunity it addresses.

## Goals / Objectives
- Goal 1
- Goal 2
- ...

## Steps / Implementation Plan
1. Step one
2. Step two
3. ...

## Acceptance Criteria
- [ ] Criterion 1 (measurable)
- [ ] Criterion 2
- [ ] Criterion 3
- [ ] All edge cases considered
- [ ] Tests pass / Documentation updated

## Requirements / Specifications
- Functional requirements
- Non-functional requirements (performance, security, etc.)
- Technical constraints

## Dependencies
- Blocking tasks: [[TASK-YYY]]
- Related tasks: [[TASK-ZZZ]]

## Risks & Mitigations
- Risk 1 → Mitigation
- Risk 2 → Mitigation

## Definition of Done (DoD)
- [ ] Code written and reviewed
- [ ] Unit & integration tests added and passing
- [ ] Documentation updated
- [ ] QA / stakeholder sign-off
- [ ] Deployed to production (if applicable)

## Additional Information
- Links to designs, Figma, API docs, etc.
- Screenshots / references

```

The h1 `Title` string must match the string of the `title` frontmatter field.

**Required sections (advisory, i2):** the kind registry records `Description` and
`Acceptance Criteria` as the required body sections for a `task`. They are
scaffolded on create and reported by `doctor`/`lint` when missing — a SOFT
advisory, never a write-time error. The remaining headings above are recommended
but optional. Overridable per project via
`[kinds.task] required-sections = [...]` in `config.toml`.

### Struct

```rust

```


