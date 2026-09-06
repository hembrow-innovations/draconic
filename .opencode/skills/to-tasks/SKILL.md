---
name: to-tasks
description: Write management task files linked from frozen slices. Use when publishing the task pool after freeze, breaking a slice into AFK or HITL tasks, or when design-tree Publish needs tasks.
---

# To tasks

Write task files using **management** conventions. This skill does not invent layout, status, or frontmatter keys.

Load **management** before any write under `.heio/`. Load **docs** for glossary terms and ADRs. A slice must already exist. Load **to-slices** first if it does not.

Do not write tasks before freeze. A `shaping` slice is not ready.

## Process

### 1. Gather context

Read the frozen slice file. Work from its Done, oracles, and the confirmed tracer-bullet list.

### 2. Draft units

Each task is one sitting, vertical, sized for a fresh context window.

- Prefactoring is its own first task and blocks the rest
- Prefer many thin tasks over few thick ones
- `mode: afk` when an agent can take it. `mode: hitl` when a human decision is required. Prefer AFK
- Wide refactors sequence as expand, then migrate, then contract. Wire `blocked_by` in that order

### 3. Show, then publish

Present a numbered list. For each task: title, slice, `mode`, `blocked_by`, and what it delivers.

If the user is present, ask about granularity, blockers, merge or split, HITL vs AFK. If they already confirmed, publish.

Copy `templates/task.md` from **management**. **management** owns `<NN>`, placement, and frontmatter.

- Path: `.heio/planning/tasks/task-<NN>-<slug>.md`
- Status: `ready` when every `blocked_by` id is `completed` or none. `draft` only while still forming. Do not use `hold`
- `mode: afk` or `mode: hitl`
- `blocked_by` lists task ids. Publish blockers first so wikilinks are real
- `sprint` and `slice` match the parent slice
- Append durable `[[task-<NN>-<slug>]]` links on the slice Pool. Never drop them

A task is the executable. Drain claims `status: ready` and `mode: afk`.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY`. Publishing tasks is not VERIFY.
