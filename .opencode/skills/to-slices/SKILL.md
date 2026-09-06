---
name: to-slices
description: Write management slice files from a settled grouping. Use when publishing slices after a planning sitting, converting confirmed tracer bullets into slice notes, or when design-tree Publish needs slices.
---

# To slices

Write slice files using **management** conventions. This skill does not invent layout, status, or frontmatter keys.

Load **management** before any write under `.heio/`. Load **docs** for glossary terms and ADRs. Load **to-tasks** in the same pass when the sitting also publishes tasks.

## Process

### 1. Gather context

Work from the confirmed tracer-bullet list. Read intent, roadmap, and the sprint `shape.md`. If a ticket is being promoted, read that ticket.

### 2. Draft vertical cuts

Each slice is a thin vertical cut, usable or learnable on its own. Not a layer.

- Prefactoring is its own first slice and blocks the rest
- Prefer many thin slices over few thick ones
- A slice you cannot demo or learn from in one sitting is two slices
- HITL vs AFK is a task facet. The slice still holds oracles

### 3. Show, then publish

Present a numbered list. For each slice: title, sprint, `blocked_by`, Done, and whether oracles can be written now.

If the user is present, ask about granularity, merge or split, and blockers. If they already confirmed, publish.

Copy `templates/slice.md` from **management**. **management** owns `<NN>`, placement, and frontmatter.

- Path: `.heio/planning/sprints/<sprint_name>/slice-<NN>-<slug>.md`
- Status: `frozen` when Done and `EXPECT:` exist. Otherwise leave `shaping` and do not publish tasks for it
- `blocked_by` lists other slice ids. Publish blockers first so wikilinks are real
- Write oracles on the file in the same pass
- Link the slice from `shape.md` Slices in
- If a ticket promoted into this slice, set that ticket `status: promoted` and wikilink both ways

Do not write tasks from this skill. Load **to-tasks** for that.

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY`. Publishing slices is not VERIFY.
