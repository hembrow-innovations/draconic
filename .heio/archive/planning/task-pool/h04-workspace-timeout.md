---
id: "h04-workspace-timeout"
title: "H04 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T16:22:45Z"
updated_at: "2026-09-04T16:51:44Z"
---

# H04 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H04 work; the host fs conformance harness stays green.

## Context

Roadmap ID **H04** (Filesystem: read / write / dirs). Review of [[s-h04]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_fs`) stayed green. If the hang comes from the H04 change, fix that filesystem read / write / dirs surface so both the workspace check and the host fs harness hold. Mark H04 `done` only when those tests are green. Not H04.01 File read (whole-file bytes + UTF-8 text; missing → typed error), H04.02 File write / append; create/truncate, H04.03 `exists` / `stat`, H04.04 Directory mkdir / readdir / rmdir / remove file, H04.05 Rename / copy / delete file, H04.06 Open handle open/read/write/seek/close, H03 path helpers (string ops; no I/O), H00 host I/O surface policy, or R02 permission grant/deny for fs. Do not re-open [[s-h04]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_fs --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_fs` still prints `test result: ok.` H04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H04), `tests/conformance/tests/host_fs.rs`, `tests/conformance/fixtures/host/fs`, `crates/draconic-backend-llvm/src/host_fs.rs`, filesystem read / write / dirs surface as needed to unhang workspace tests after H04

## Links

[[s-h04-workspace-timeout]] [[ticket-150-h04-workspace-timeout]] [[s-h04]]
