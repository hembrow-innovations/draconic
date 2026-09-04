---
id: "h02-workspace-timeout"
title: "H02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T16:05:02Z"
updated_at: "2026-09-04T16:37:06Z"
---

# H02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H02 work; the host stdio conformance harness stays green.

## Context

Roadmap ID **H02** (Stdio: stdout / stderr / stdin). Review of [[s-h02]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_stdio`) stayed green. If the hang comes from the H02 change, fix that stdout/stderr/stdin surface so both the workspace check and the host stdio harness hold. Mark H02 `done` only when those tests are green. Not H02.01 stdout write string + newline; bytes via `Uint8Array`, H02.02 stderr write, H02.03 stdin read line or bounded bytes, H00 host I/O surface policy, H01 process args/env/exit, or L06 leveled logger on stderr/stdout. Do not re-open [[s-h02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_stdio --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_stdio` still prints `test result: ok.` H02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H02), `tests/conformance/tests/host_stdio.rs`, `tests/conformance/fixtures/host/stdio`, `crates/draconic-runtime`, `crates/draconic-backend-llvm/src/host_stdio.rs`, host stdout/stderr/stdin as needed to unhang workspace tests after H02

## Links

[[s-h02-workspace-timeout]] [[ticket-148-h02-workspace-timeout]] [[s-h02]]
