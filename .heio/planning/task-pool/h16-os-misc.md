---
id: "h16-os-misc"
title: "H16 OS misc"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:33:21Z"
updated_at: "2026-09-02T13:33:21Z"
---

# H16 OS misc

## Done

ROADMAP H16 is implemented test-first on both targets: a Program can get and change cwd, read `hostname()` / `osType()` / `osArch()`, read `tempDir()` / `homeDir()`, and on native sleep / yield for timer tests; `host/os` fixtures are green and H16 is `done`.

## Context

Roadmap ID **H16** (OS misc). H16.01–H16.04 already land cwd get + chdir, hostname / OS type / arch strings, temp dir + home dir paths, and native OS sleep / yield for timer tests; this sitting unifies them as one honest OS-misc surface on both targets. Tests under `tests/conformance` fixtures `host/os`. Harness `tests/conformance/tests/host_os.rs`. Mark H16 `done` only when those tests are green. Not H16.01–H16.04 as separate atoms, H01, H03, H05, or H00.

## Verify

`cargo test -p draconic-conformance --test host_os` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H16 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H16), `tests/conformance/fixtures/host/os`, `tests/conformance/tests/host_os.rs`, `crates/draconic-backend-llvm/src/host_os.rs`, `crates/draconic-runtime`, both-target OS-misc paths as needed for the parent surface

## Links

[[s-h16]] [[ticket-47-h16-os-misc]]
