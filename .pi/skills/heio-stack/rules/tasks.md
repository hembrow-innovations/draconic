---
title: Tasks
impact: HIGH
tags: [tasks]
---

# Tasks

Tasks are files in `.heio/planning/task-pool/` following `templates/pool-task.md`. One markdown file per task. The file stem is the id.

Each file keeps `id`, `title`, `kind`, `status`, `mode`, `blocked-by`, `tags`, timestamps, **Done**, **Context**, **Verify** with `scope:`, and optional **Links**.

**Context** is an agent brief: current vs desired behavior, interfaces, out of scope. Durable enough that an AFK agent can take it after the planning sitting ends.

`blocked-by` names gating task-pool ids. The frontier is `ready` AFK tasks whose blockers are `completed` or none.

The slice keeps durable `[[id]]` links to those task-pool ids. Links are never dropped. When a task is `completed`, the file moves to `.heio/archive/planning/task-pool/`; the slice still names the id.

A planning sitting publishes the pool after freeze. Inbound product work becomes a ticket first. Triage may add a task-pool file and link it from the slice.
