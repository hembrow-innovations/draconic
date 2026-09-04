---
id: "h15-workspace-timeout"
title: "H15 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:44:37Z"
updated_at: "2026-09-04T18:02:34Z"
---

# H15 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H15 work; the host process subprocess conformance harness stays green.

## Context

Roadmap ID **H15** (Subprocess). Review of [[s-h15]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_process`) stayed green. If the hang comes from the H15 change, fix that spawn/run, capture/kill, and native async wait surface so both the workspace check and the host process subprocess harness hold. Mark H15 `done` only when those tests are green. Not H15.01 spawn/run argv, env subset, cwd, wait exit code, H15.02 capture stdout/stderr; write stdin; kill child, H15.03 async subprocess exit via job queue / Promise, H01 process args, env, exit, H14 signals, H16 OS misc, H00 host I/O surface policy, or js subprocess APIs / a Node polyfill. Do not re-open [[s-h15]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_process --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_process` still prints `test result: ok.` H15 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H15), `tests/conformance/tests/host_process.rs`, `tests/conformance/fixtures/host/process`, `crates/draconic-backend-llvm/src/host_subprocess.rs`, `crates/draconic-runtime`, spawn/run, capture/kill, and native async wait surface as needed to unhang workspace tests after H15

## Links

[[s-h15-workspace-timeout]] [[ticket-161-h15-workspace-timeout]] [[s-h15]]
