---
id: "h03-workspace-timeout"
title: "H03 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T16:13:12Z"
updated_at: "2026-09-04T16:41:59Z"
---

# H03 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H03 work; the host path conformance harness stays green.

## Context

Roadmap ID **H03** (Path helpers (string ops; no I/O)). Review of [[s-h03]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_path`) stayed green. If the hang comes from the H03 change, fix that path-helper surface (string ops; no I/O) so both the workspace check and the host path harness hold. Mark H03 `done` only when those tests are green. Not H03.01 `path.join` / `path.normalize`, H03.02 `dirname` / `basename` / `extname` / `isAbsolute`, H03.03 `path.resolve` relative to cwd, H04 filesystem read / write / dirs, H16 OS misc, or H00 host I/O surface policy. Do not re-open [[s-h03]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_path --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_path` still prints `test result: ok.` H03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H03), `tests/conformance/tests/host_path.rs`, `tests/conformance/fixtures/host/path`, `crates/draconic-backend-llvm/src/host_path.rs`, path-helper surface as needed to unhang workspace tests after H03

## Links

[[s-h03-workspace-timeout]] [[ticket-149-h03-workspace-timeout]] [[s-h03]]
