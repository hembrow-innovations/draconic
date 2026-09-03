---
id: "h04-filesystem-read-write-dirs"
title: "H04 filesystem read / write / dirs"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:24:30Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H04 filesystem read / write / dirs

## Done

ROADMAP H04 is implemented test-first on both targets: whole-file read (bytes and UTF-8 text; missing → typed error), write/append/create/truncate, `exists` / `stat` (size, isFile, isDir, mtime), mkdir (optional recursive) / readdir / rmdir / remove file, rename / copy / delete file, and native fd-like open/read/write/seek/close; `host/fs` fixtures are green and H04 is `done`.

## Context

Roadmap ID **H04** (Filesystem: read / write / dirs). H04.01–H04.06 already land whole-file read/write, exists/stat, mkdir/readdir/rmdir, rename/copy/delete, and native open handles; this sitting unifies them as one honest filesystem surface on both targets. Tests under `tests/conformance` fixtures `host/fs`. Harness `tests/conformance/tests/host_fs.rs`. Mark H04 `done` only when those tests are green. Not H03, H00, or R02.

## Verify

`cargo test -p draconic-conformance --test host_fs` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H04), `tests/conformance/fixtures/host/fs`, `tests/conformance/tests/host_fs.rs`, `crates/draconic-backend-llvm/src/host_fs.rs`, `crates/draconic-runtime`, js/native fs paths as needed for the parent surface

## Links

[[s-h04]] [[ticket-35-h04-filesystem-read-write-dirs]]
