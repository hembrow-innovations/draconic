---
id: overview-planning
created_at: 2026-07-26
updated_at: 2026-07-26
area: planning
title: "Planning"
description: "Vault tracker: issues, plans, and tasks (idea = issue tagged idea)."
tags: [planning, index]
---

# Planning

Issue tracker and second-brain workspace (replaces GitHub Issues / `.scratch/`).
Workflow: `issue → plan/tasks → execute → review → new issues` (**planning-workflow**
skill). Language completeness: still [[ROADMAP]] + **draconic-loop**.

## Folders

- **Issues** — `planning/issues/` · closed → `issues/closed/` · `issues-<N>-<slug>.md`
- **Plans** — `planning/plans/` · completed → `plans/completed/` · `plans-<N>-<slug>.md`
- **Tasks** — `planning/tasks/` · completed → `tasks/completed/` · only when ≥2 units
- **Ideas** — tag `idea` on a low-priority issue (no `ideas/` folder)
- **Out of scope** — `planning/out-of-scope/` (rejected directions)

## Id allocation

```sh
node scripts/planning-next-id.mjs
node scripts/planning-check-ids.mjs
```

## Guides

- [[issue-tracker]]
- [[triage-labels]]
