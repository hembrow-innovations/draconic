---
id: "s-l02-01-workspace-timeout"
title: "L02.01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:49:26Z"
updated_at: "2026-09-04T19:59:20Z"
claimed-by: 691e677c-2a3f-452e-bd39-b15f06dbbd18
---

# L02.01 workspace tests finish

## Why

Review of [[s-l02-01]] left ROADMAP L02.01 unfinished: O1 (`stdlib_collections`) held, but O2 `cargo test --workspace` timed out at 120s. The stdlib location still needs the L02.01 Loop to leave the workspace green, not only the collections conformance fixtures for `groupBy` / `chunk` (or designed names) on arrays.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L02.01 work. The stdlib collections conformance tests stay green. If the hang comes from the L02.01 change, fix that groupBy/chunk (or designed names) surface so both the workspace check and those fixtures hold. Mark L02.01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l02-01]]**: that slice stays sealed `failed`
- **L02 parent remainder**: umbrella collections helpers row (groupBy/chunk/Deque together)
- **L02.02**: Deque (or designed): push/pop both ends
- **E-cluster Array/Map/Set**: ECMA `Object.groupBy` / `Map.groupBy` and other builtins this stdlib surface must not duplicate
- **L01**: encoding
- **L03**: crypto SHA-256 / CSPRNG
- **L07**: flags / CLI parse

## Oracle checklist

- [x] O1: workspace tests finish after the L02.01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_collections --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=c950385eaa7d8e7e bytes=94742 at=2026-09-04T19:59:02.469Z

- [x] O2: L02.01 groupBy and chunk (or designed names) fixtures stay locked by the stdlib collections conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_collections
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=932d9648eb0c63c3 bytes=3172 at=2026-09-04T19:59:03.438Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l02-01-workspace-timeout]]`

## See also

ROADMAP.md L02.01, `tests/conformance/tests/stdlib_collections.rs`, `tests/conformance/fixtures/stdlib/collections`, CONTEXT.md, [[stdlib]], [[s-l02-01]], [[ticket-180-l02-01-workspace-timeout]].
