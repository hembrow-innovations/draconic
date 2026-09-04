---
id: "s-k04-workspace-timeout"
title: "K04 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:35:24Z"
updated_at: "2026-09-04T18:48:10Z"
claimed-by: 48e0454a-1c2e-456a-bda8-cddba66d292e
---

# K04 workspace tests finish

## Why

Review of [[s-k04]] left ROADMAP K04 unfinished: O1 (`draconic-pkg` resolve) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K04 Loop to leave the workspace green, not only the semver-tag → commit OID fail-closed crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K04 work. The `draconic-pkg` resolve tests for version req against semver git tags → highest matching commit OID, fail-closed diagnostics (no match / non-semver-only / empty), and direct-deps → lock pins (v1: direct only) stay green. If the hang comes from the K04 change, fix that version-resolve surface so both checks hold. Mark K04 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k04]]**: that slice stays sealed `failed`
- **K04.01**: Resolve version req against git tags; highest matching semver (already `done`)
- **K04.02**: Fail closed: no match / non-semver-only / empty → diagnostic (already `done`)
- **K04.03**: Resolve direct-deps set → lock pins (v1: direct only) (already `done`)
- **K03**: Module cache layout / git clone
- **K05**: CLI `draconic get` / `draconic mod tidy`
- **K02**: Lockfile (`draconic.lock`) parse/write surface
- **K08**: Integrity verify lock hashes; refuse tampered cache

## Oracle checklist

- [x] O1: workspace tests finish after the K04 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline resolve
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=cff9891af392b4c1 bytes=93954 at=2026-09-04T18:47:58.127Z

- [x] O2: K04 version resolve (semver tag → commit OID; fail closed) stays locked by the draconic-pkg resolve tests
  CHECK: cargo test -p draconic-pkg resolve
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=3e8ea8524dabb92a bytes=3052 at=2026-09-04T18:47:59.589Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k04-workspace-timeout]]`

## See also

ROADMAP.md K04, `crates/draconic-pkg/src/resolve.rs`, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k04]], [[ticket-168-k04-workspace-timeout]].
