---
id: "h15-subprocess"
title: "H15 Subprocess"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:31:47Z"
updated_at: "2026-09-02T13:31:47Z"
---

# H15 Subprocess

## Done

ROADMAP H15 is implemented test-first on both targets: a Program can spawn/run a child with argv, env subset, and cwd and wait for an exit code; capture stdout/stderr, write stdin, and kill the child; and on native await child exit through the job queue / Promise; `host/process` subprocess fixtures are green and H15 is `done`.

## Context

Roadmap ID **H15** (Subprocess). H15.01–H15.03 already land spawn/run (argv, env subset, cwd, wait exit code), capture stdout/stderr plus stdin write and kill, and native async wait via the job queue; this sitting unifies them as one honest subprocess surface on both targets. Tests under `tests/conformance` fixtures `host/process` (subprocess). Harness `tests/conformance/tests/host_process.rs`. Mark H15 `done` only when those tests are green. Not H15.01–H15.03 as separate atoms, H01, H14, H16, or H00.

## Verify

`cargo test -p draconic-conformance --test host_process` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H15 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H15), `tests/conformance/fixtures/host/process`, `tests/conformance/tests/host_process.rs`, `crates/draconic-backend-llvm/src/host_subprocess.rs`, `crates/draconic-runtime`, both-target subprocess paths as needed for the parent surface

## Links

[[s-h15]] [[ticket-46-h15-subprocess]]
