---
id: "h03-path-helpers-string-ops-no-i-o"
title: "H03 Path helpers (string ops; no I/O)"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:15:29Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H03 Path helpers (string ops; no I/O)

## Done

ROADMAP H03 is implemented test-first on both targets: a Program can `path.join` / `path.normalize` (POSIX + Windows-aware as designed), `dirname` / `basename` / `extname` / `isAbsolute`, and `path.resolve` relative to cwd, all as string ops with no filesystem I/O; `host/path` fixtures are green and H03 is `done`.

## Context

Roadmap ID **H03** (Path helpers (string ops; no I/O)). H03.01–H03.03 already land join/normalize, dirname/basename/extname/isAbsolute, and resolve-relative-to-cwd; this sitting unifies them as one honest path surface on both targets. Tests under `tests/conformance/host/path`. Harness `tests/conformance/tests/host_path.rs`. Mark H03 `done` only when those tests are green. Not H03.01, H03.02, H03.03, H04, H16, or H00.

## Verify

`cargo test -p draconic-conformance --test host_path` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H03), `tests/conformance/fixtures/host/path`, `tests/conformance/tests/host_path.rs`, `crates/draconic-backend-llvm/src/host_path.rs`, js/native path-helper paths as needed for the parent surface

## Links

[[s-h03]] [[ticket-34-h03-path-helpers-string-ops-no]]
