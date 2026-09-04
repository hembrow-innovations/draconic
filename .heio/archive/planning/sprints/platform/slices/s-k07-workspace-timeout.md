---
id: "s-k07-workspace-timeout"
title: "K07 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:46:29Z"
updated_at: "2026-09-04T19:09:42Z"
claimed-by: 8de8d5f1-3926-4fe0-896e-d3471cbc0357
---

# K07 workspace tests finish

## Why

Review of [[s-k07]] left ROADMAP K07 unfinished: O1 (`draconic-cli` build) and O2 (`draconic-pkg` ensure) held, but O3 `cargo test --workspace` timed out at 120s. The packages location still needs the K07 Loop to leave the workspace green, not only the auto-fetch / `--offline` / lock-pin crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K07 work. The `draconic-cli` build tests for auto-fetch of missing locked cache entries, `--offline` cache-only with a fixit on miss, and lock-pin preference (no float when a lock is present) stay green, as do the `draconic-pkg` ensure tests for that same cache/offline/pin surface. If the hang comes from the K07 change, fix that auto-fetch / `--offline` build surface so both the workspace check and those crate tests hold. Mark K07 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k07]]**: that slice stays sealed `failed`
- **K07.01**: `draconic build` auto-fetches missing locked cache entries (already `done`)
- **K07.02**: `draconic build --offline`: cache only; fixit on miss (already `done`)
- **K07.03**: Build prefers lock pins; does not float versions when lock present (already `done`)
- **K02**: Lockfile (`draconic.lock`) resolved pins
- **K03**: Module cache layout / git clone
- **K05**: CLI `draconic get` / `draconic mod tidy`
- **K08**: Integrity verify lock hashes; refuse tampered cache

## Oracle checklist

- [x] O1: workspace tests finish after the K07 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-cli --offline --test build && cargo test -p draconic-pkg --offline ensure
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=4b1fefd98219b21b bytes=95959 at=2026-09-04T19:09:19.047Z

- [x] O2: K07 auto-fetch, `--offline` miss/hit, and lock-pin preference stay locked by the draconic-cli build tests
  CHECK: cargo test -p draconic-cli --test build
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=28cfa6771e96678a bytes=3842 at=2026-09-04T19:09:22.146Z

- [x] O3: K07 ensure-locked cache/offline/pin behaviour stays locked by the draconic-pkg ensure tests
  CHECK: cargo test -p draconic-pkg ensure
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=8f0a6aa0e95063f3 bytes=1215 at=2026-09-04T19:09:22.768Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k07-workspace-timeout]]`

## See also

ROADMAP.md K07, `crates/draconic-cli/tests/build.rs`, `crates/draconic-pkg/src/ensure.rs`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k07]], [[ticket-170-k07-workspace-timeout]].
