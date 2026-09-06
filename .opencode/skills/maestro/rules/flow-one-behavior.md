---
title: One user-visible journey per flow
impact: HIGH
impactDescription: kitchen-sink YAML hides which step failed
tags: [flow, assert]
---

## One user-visible journey per flow

One reason to go red. Sign-in, then a separate flow for create-task. Share setup with `runFlow`.

**Incorrect:** One YAML that signs in, creates three records, opens settings, and signs out.

**Correct:** `signin.yaml` asserts the home root. `create-task.yaml` runs the sign-in subflow, then creates one task, then asserts that row.

Notes: Tags can still group a folder run. See `flow-runflow-setup`.
