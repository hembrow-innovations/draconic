---
id: "s-k05-workspace-timeout"
title: "K05 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:39:53Z"
updated_at: "2026-09-04T18:51:10Z"
claimed-by: c9a053ed-2587-41f3-8554-189497c0383b
---

# K05 workspace tests finish

## Why

Review of [[s-k05]] left ROADMAP K05 unfinished: O1 (`draconic-cli` get) and O2 (`draconic-cli` mod_tidy) held, but O3 `cargo test --workspace` timed out at 120s. The packages location still needs the K05 Loop to leave the workspace green, not only the `draconic get` / `draconic mod tidy` CLI tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K05 work. The `draconic-cli` get tests for `draconic get <module_path>@<ver>` (fetch, update manifest+lock+cache) and the `draconic-cli` mod_tidy tests for `draconic mod tidy` (lock matches manifest; fetch missing; prune unused) stay green. If the hang comes from the K05 change, fix that get/tidy CLI surface so both the workspace check and those crate tests hold. Mark K05 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k05]]**: that slice stays sealed `failed`
- **K05.01**: `draconic get <module_path>@<ver>`: fetch, update manifest+lock+cache (already `done`)
- **K05.02**: `draconic mod tidy`: lock matches manifest; fetch missing; prune unused (already `done`)
- **K01**: Manifest (`draconic.toml`)
- **K02**: Lockfile (`draconic.lock`)
- **K03**: Module cache layout / git clone
- **K04**: Version resolve (semver tag → commit OID)
- **K07**: Build integration auto-fetch / offline
- **K08**: Integrity verify lock hashes; refuse tampered cache

## Oracle checklist

- [x] O1: workspace tests finish after the K05 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --offline --test get && cargo test -p draconic-cli --offline --test mod_tidy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=22e4d7129b39f213 bytes=96959 at=2026-09-04T18:50:54.217Z

- [x] O2: K05 `draconic get` stays locked by the draconic-cli get tests
  CHECK: cargo test -p draconic-cli --test get
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=31db1157ba64998c bytes=2968 at=2026-09-04T18:50:55.285Z

- [x] O3: K05 `draconic mod tidy` stays locked by the draconic-cli mod_tidy tests
  CHECK: cargo test -p draconic-cli --test mod_tidy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=de3f312f6515f2eb bytes=3035 at=2026-09-04T18:50:55.823Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k05-workspace-timeout]]`

## See also

ROADMAP.md K05, `crates/draconic-cli/tests/get.rs`, `crates/draconic-cli/tests/mod_tidy.rs`, `crates/draconic-pkg/src/get.rs`, `crates/draconic-pkg/src/tidy.rs`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k05]], [[ticket-169-k05-workspace-timeout]].
