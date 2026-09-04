---
id: "s-k08-workspace-timeout"
title: "K08 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:57:28Z"
updated_at: "2026-09-04T19:11:29Z"
claimed-by: 4b427cfd-4d67-49b0-bf88-29c4b1981236
---

# K08 workspace tests finish

## Why

Review of [[s-k08]] left ROADMAP K08 unfinished: O1 (`draconic-pkg` hash) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K08 Loop to leave the workspace green, not only the lock-hash verify / refuse-tampered-cache crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K08 work. The `draconic-pkg` hash tests for recomputing canonical tree SHA-256 against the lock pin and refusing a mismatched OID or content hash (no silent wrong tree) stay green. If the hang comes from the K08 change, fix that verify-lock-hashes / refuse-tampered-cache surface so both the workspace check and those crate tests hold. Mark K08 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k08]]**: that slice stays sealed `failed`
- **K08.01**: Recompute tree SHA-256; match lock or hard-fail (already `done`)
- **K08.02**: Refuse mismatched OID/hash; no silent wrong tree (already `done`)
- **K02**: Lockfile (`draconic.lock`) resolved pins
- **K03**: Module cache layout / git clone
- **R03 / R03.01 / R03.02**: Integration supply-chain tests once K08 lands
- **K09**: E2E temp git dep + consumer Program

## Oracle checklist

- [x] O1: workspace tests finish after the K08 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline hash
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=be9bed8f69ee113d bytes=93298 at=2026-09-04T19:11:19.218Z

- [x] O2: K08 lock-hash verify and tampered-cache refuse stay locked by the draconic-pkg hash tests
  CHECK: cargo test -p draconic-pkg hash
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=fc76015a843b7107 bytes=2396 at=2026-09-04T19:11:19.512Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k08-workspace-timeout]]`

## See also

ROADMAP.md K08, `crates/draconic-pkg/src/hash.rs`, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k08]], [[ticket-171-k08-workspace-timeout]].
