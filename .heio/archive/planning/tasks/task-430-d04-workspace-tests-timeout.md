---
id: "task-430-d04-workspace-tests-timeout"
title: "D04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked_by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-06T18:00:00Z"
sprint: "platform"
---

# D04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the oracle CHECK budget. cross_compile, cross_compile_matrix, and release_artifact stay green against `.yml.disabled`.

## Context

Reviewer miss on [[slice-255-d04-workspace-tests]]. O1 matched EXPECT but `exit=timeout`. Not a product fail. Not a new D04 Loop atom. Do not restore live `.yml` names. D04 / D04.02 / D01.01 stay `done`.

## Verify

`cargo test --workspace` exit 0 with `test result: ok.` The three D04 readers still print `test result: ok.`

scope: workspace test runtime as needed so O1 finishes in budget; do not restore `.yml` names

## Links

[[slice-254-d04-workspace-tests-timeout]] [[ticket-191-d04-workspace-tests-timeout]]

## Gauntlet

- **round 1**: `cargo test --workspace` — win. Workspace finished with `test result: ok.` (161 ok lines, exit 0, no hang / no exit 101). Native runtime C fallback in `draconic-runtime` covers stale `CARGO_MANIFEST_DIR`. ROADMAP row remains `done`.
