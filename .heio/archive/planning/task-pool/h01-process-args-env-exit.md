---
id: "h01-process-args-env-exit"
title: "H01 process args, env, exit"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:22:48Z"
updated_at: "2026-09-02T20:55:00Z"
---

# H01 process args, env, exit

## Done

ROADMAP H01 is implemented test-first on both targets: argv as a string array, env get/set/delete (missing → undefined), `exit(code)` / exitCode (default 0), and read-only pid/ppid; `host/process` fixtures are green and H01 is `done`.

## Context

Roadmap ID **H01** (Process: args, env, exit). H01.01–H01.04 already land program args, env get/set/delete, `exit(code)` / exitCode, and pid/ppid; this sitting unifies them as one honest process control surface on both targets. Tests under `tests/conformance` fixtures `host/process`. Harness `tests/conformance/tests/host_process.rs`. Mark H01 `done` only when those tests are green. Not H00, H02, H14, or H15.

## Verify

`cargo test -p draconic-conformance --test host_process` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H01), `tests/conformance/fixtures/host/process`, `tests/conformance/tests/host_process.rs`, `crates/draconic-runtime`, js/native process paths as needed for the parent surface

## Links

[[s-h01]] [[ticket-32-h01-process-args-env-exit]]
