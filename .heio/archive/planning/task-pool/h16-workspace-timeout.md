---
id: "h16-workspace-timeout"
title: "H16 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T17:51:43Z"
updated_at: "2026-09-04T18:08:30Z"
---

# H16 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H16 work; the host os conformance harness stays green.

## Context

Roadmap ID **H16** (OS misc). Review of [[s-h16]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_os`) stayed green. If the hang comes from the H16 change, fix that cwd get + chdir, hostname / OS type / arch, temp/home dir, and native sleep / yield surface so both the workspace check and the host os conformance harness hold. Mark H16 `done` only when those tests are green. Not H16.01 cwd get + chdir, H16.02 hostname / OS type / arch strings, H16.03 temp dir + home dir paths, H16.04 OS sleep / yield for timer tests, H01 process args, env, exit, H03 path helpers, H05 time, clock, timers, H00 host I/O surface policy, or js OS-misc APIs / a Node polyfill beyond the existing both-targets fixtures. Do not re-open [[s-h16]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_os --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_os` still prints `test result: ok.` H16 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H16), `tests/conformance/tests/host_os.rs`, `tests/conformance/fixtures/host/os`, `crates/draconic-backend-llvm/src/host_os.rs`, `crates/draconic-runtime`, cwd get + chdir, hostname / OS type / arch, temp/home dir, and native sleep / yield surface as needed to unhang workspace tests after H16

## Links

[[s-h16-workspace-timeout]] [[ticket-162-h16-workspace-timeout]] [[s-h16]]
