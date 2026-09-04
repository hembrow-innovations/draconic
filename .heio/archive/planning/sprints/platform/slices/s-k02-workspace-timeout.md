---
id: "s-k02-workspace-timeout"
title: "K02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:18:37Z"
updated_at: "2026-09-04T18:29:53Z"
claimed-by: 87cdd855-836f-49f9-b09d-1759e534a6cc
---

# K02 workspace tests finish

## Why

Review of [[s-k02]] left ROADMAP K02 unfinished: O1 (`draconic-pkg` lock) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K02 Loop to leave the workspace green, not only the lockfile parse/write/serialize crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K02 work. The `draconic-pkg` lock tests for `draconic.lock` resolved pins (path + version + git URL + commit OID + content hash SHA-256, parse/write reject-malformed, stable serialize) stay green. If the hang comes from the K02 change, fix that lockfile surface so both checks hold. Mark K02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k02]]**: that slice stays sealed `failed`
- **K02.01**: Lock entry: path + version + git URL + commit OID + content hash SHA-256 (already `done`)
- **K02.02**: Parse/write lock; reject malformed (already `done`)
- **K02.03**: Stable lock serialize: sorted paths; byte-identical rewrite when unchanged (already `done`)
- **K01**: Manifest (`draconic.toml`)
- **K03**: Module cache layout / git clone
- **K04**: Version resolve (semver tag / commit)
- **K08**: Integrity verify lock hashes; refuse tampered cache

## Oracle checklist

- [x] O1: workspace tests finish after the K02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline lock
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=0ec878a7482a0ea0 bytes=95056 at=2026-09-04T18:29:34.959Z

- [x] O2: K02 lockfile resolved pins stay locked by the draconic-pkg lock tests
  CHECK: cargo test -p draconic-pkg lock
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=dbf81c26a260cb60 bytes=4154 at=2026-09-04T18:29:36.057Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k02-workspace-timeout]]`

## See also

ROADMAP.md K02, `crates/draconic-pkg/src/lock.rs`, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k02]], [[ticket-166-k02-workspace-timeout]].
