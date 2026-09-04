---
title: Task-pool statuses
impact: HIGH
tags: [pool]
---

# Task-pool statuses

`.heio/planning/task-pool/` holds one markdown file per task. Copy `templates/pool-task.md`. Statuses, in order:

`draft` → `ready` → `claimed` → `implemented` → `completed`

- **draft**: the task exists. Done, Context, or Verify may still be thin.
- **ready**: the task is specified enough to take. Done, Context, Verify, `scope:`, `mode:`, and `blocked-by` are in place. Planning publishes here. `mode: afk` plus empty (or completed) `blocked-by` is agent-grabbable.
- **claimed**: the task is in progress.
- **implemented**: the change is in place. Verify is still open.
- **completed**: Done and Verify hold. The file moves to `.heio/archive/planning/task-pool/`.

`mode: afk` an agent can take alone. `mode: hitl` waits for a human. Drain skips HITL.

The slice that links the id keeps that `[[id]]` after the file moves.
