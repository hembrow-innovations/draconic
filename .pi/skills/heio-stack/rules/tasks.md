---
title: Tasks
impact: HIGH
tags: [tasks]
---

# Tasks

TDD is **how you do a build task**. Oracles are **how you prove the slice**. Do not flatten them. Do not make every test an oracle. Do not skip TDD because the oracle will catch it later.

- **Task loop**: red → green → refactor. Seconds to minutes. Design tool. Load **tdd**.
- **Oracle loop**: pending → met. Hours to days. Evidence tool. Load **oracle**.

Only the active slice has `tasks.md`. Copy `templates/slice-tasks.md` when the slice turns `active`. **heio-tasker** writes that file from the frozen spec + oracles. Incoming work becomes a ticket first; triage may append a task.

Each task names the oracle it serves (`fits: O1`) and a one-line done. Checkbox is the status. `[x]` when the builder’s red-green loop held. `DROPPED: <reason>` when the task leaves the slice — and that reason is a ticket id or “drop from sprint.”
