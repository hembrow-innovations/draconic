---
title: Tasks
impact: HIGH
tags: [tasks]
---

# Tasks

Tasks are files in `.heio/planning/task-pool/` following `templates/pool-task.md`. One markdown file per task. The file stem is the id.

Each file keeps `id`, `title`, `kind`, `status`, `tags`, timestamps, **Done**, **Context**, **Verify** with `scope:`, and optional **Links**.

The slice keeps durable `[[id]]` links to those task-pool ids. Links are never dropped. When a task is `completed`, the file moves to `.heio/archive/planning/task-pool/`; the slice still names the id.

Inbound product work becomes a ticket first. Triage may add a task-pool file and link it from the slice.
