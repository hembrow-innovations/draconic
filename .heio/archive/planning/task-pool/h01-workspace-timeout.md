---
id: "h01-workspace-timeout"
title: "H01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:54:53Z"
updated_at: "2026-09-04T16:32:42Z"
---

# H01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H01 work; the host process conformance harness stays green.

## Context

Roadmap ID **H01** (Process: args, env, exit). Review of [[s-h01]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_process`) stayed green. If the hang comes from the H01 change, fix that process args/env/exit surface so both the workspace check and the host process harness hold. Mark H01 `done` only when those tests are green. Not H01.01 program args as string array, H01.02 env get/set/delete, H01.03 `exit(code)` / exitCode, H01.04 `pid` + `ppid`, H00 host I/O surface policy, H02 stdio, H14 signals, or H15 subprocess spawn/run/capture. Do not re-open [[s-h01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_process --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_process` still prints `test result: ok.` H01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H01), `tests/conformance/tests/host_process.rs`, `tests/conformance/fixtures/host/process`, `crates/draconic-runtime`, host process args/env/exit as needed to unhang workspace tests after H01

## Links

[[s-h01-workspace-timeout]] [[ticket-147-h01-workspace-timeout]] [[s-h01]]
