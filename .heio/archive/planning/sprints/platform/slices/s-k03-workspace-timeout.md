---
id: "s-k03-workspace-timeout"
title: "K03 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:26:46Z"
updated_at: "2026-09-04T18:41:11Z"
claimed-by: 2d5c938f-7745-4f5c-aed3-5dfa1a59defa
---

# K03 workspace tests finish

## Why

Review of [[s-k03]] left ROADMAP K03 unfinished: O1 (`draconic-pkg` cache) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K03 Loop to leave the workspace green, not only the module cache layout / git clone-fetch / OID checkout crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K03 work. The `draconic-pkg` cache tests for module cache layout keyed by module path + commit OID, git clone/fetch into the VCS store, checkout of a pinned OID, and SHA-256 over the canonical package tree stay green. If the hang comes from the K03 change, fix that module cache surface so both checks hold. Mark K03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k03]]**: that slice stays sealed `failed`
- **K03.01**: Cache layout keyed by module path + commit OID (already `done`)
- **K03.02**: git clone/fetch into cache (HTTPS; fixture repos in tests) (already `done`)
- **K03.03**: Checkout pinned OID; cache hit skips network (already `done`)
- **K03.04**: Content hash SHA-256 over canonical package tree (already `done`)
- **K01**: Manifest (`draconic.toml`)
- **K02**: Lockfile (`draconic.lock`) parse/write surface
- **K04**: Version resolve (semver tag / commit)
- **K08**: Integrity verify lock hashes; refuse tampered cache

## Oracle checklist

- [x] O1: workspace tests finish after the K03 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline cache
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=45f93ae07b136206 bytes=93516 at=2026-09-04T18:40:52.360Z

- [x] O2: K03 module cache layout, git clone/fetch, and OID checkout stay locked by the draconic-pkg cache tests
  CHECK: cargo test -p draconic-pkg cache
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=2614634c2de2abbc bytes=2614 at=2026-09-04T18:40:53.096Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k03-workspace-timeout]]`

## See also

ROADMAP.md K03, `crates/draconic-pkg/src/cache.rs`, `crates/draconic-pkg/src/hash.rs`, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k03]], [[ticket-167-k03-workspace-timeout]].
