---
id: "task-217-workspace-test-budget"
title: "Workspace tests finish inside the oracle budget"
kind: task
status: completed
mode: afk
blocked_by: []
tags: []
created_at: "2026-09-06T04:39:34Z"
updated_at: "2026-09-06T18:00:00Z"
sprint: "platform"
slice: "slice-216-workspace-test-budget"
---

# Workspace tests finish inside the oracle budget

## Blocked by

None.

## Done

`cargo test --workspace` exits 0 and prints `test result: ok.` inside the 600s oracle CHECK budget. `release_artifact` still prints `test result: ok.` against `.yml.disabled`.

## Context

Current: warm `cargo test --workspace` is green (161 `test result: ok.` lines, exit 0) but wall clock is ~829s because 161 full-debug test binaries spend minutes on startup/page-ins (user 265s, sys 240s, test-harness time only ~290s). Desired: test profile uses line-tables-only debuginfo and no incremental cache so the same suite finishes inside 600s and `target/` stays under 10GB. Out of scope: restoring live `.yml` names; new D/L/P/R Loop atoms; E17.02 / E18.44 remainder.

## Verify

`cargo test --workspace` exit 0 with `test result: ok.` in under 600s. `cargo test -p draconic-integration-tests --test release_artifact` exit 0 with `test result: ok.`

scope: `Cargo.toml` `[profile.test]` only

## Links

[[slice-216-workspace-test-budget]] [[ticket-204-d04-workspace-tests-timeout]]

## Gauntlet

- **round 1**: `cargo test --workspace` then `cargo test -p draconic-integration-tests --test release_artifact` — win. Clean `target/debug` then workspace 161 `test result: ok.` lines, exit 0, wall 377.83s (under 600s). `target/` 2.1G. release_artifact 2 passed, 0.69s. ROADMAP D/L/P/R rows remain `done`.
