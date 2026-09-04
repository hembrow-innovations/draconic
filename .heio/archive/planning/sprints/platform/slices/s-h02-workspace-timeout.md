---
id: "s-h02-workspace-timeout"
title: "H02 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T16:03:28Z"
updated_at: "2026-09-04T16:41:15Z"
claimed-by: bbaffd29-fab7-4c8c-b7d4-818e4ee71c0d
---

# H02 workspace tests finish

## Why

Review of [[s-h02]] left ROADMAP H02 unfinished: O1 (`host_stdio`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H02 Loop to leave the workspace green, not only the host stdio conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H02 work. The host stdio conformance harness stays green. If the hang comes from the H02 change, fix that stdout/stderr/stdin surface so both checks hold. Mark H02 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h02]]**: that slice stays sealed `failed`
- **H02.01**: stdout write string + newline; bytes via `Uint8Array` (already `done`)
- **H02.02**: stderr write (already `done`)
- **H02.03**: stdin read line or bounded bytes (already `done`)
- **H00**: host I/O surface policy
- **H01**: process args, env, exit
- **L06**: leveled logger on stderr/stdout

## Oracle checklist

- [x] O1: workspace tests finish after the H02 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_stdio --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=e702d49df8e6133f bytes=102605 at=2026-09-04T16:41:01.180Z

- [x] O2: H02 stdout, stderr, and stdin stay locked by the host stdio conformance tests
  CHECK: cargo test -p draconic-conformance --test host_stdio
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=54c2e6bff947d390 bytes=3543 at=2026-09-04T16:41:03.605Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h02-workspace-timeout]]`

## See also

ROADMAP.md H02, `tests/conformance/tests/host_stdio.rs`, `tests/conformance/fixtures/host/stdio`, `crates/draconic-runtime`, `crates/draconic-backend-llvm/src/host_stdio.rs`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h02]], [[ticket-148-h02-workspace-timeout]].
