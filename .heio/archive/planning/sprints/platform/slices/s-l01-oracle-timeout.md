---
id: "s-l01-oracle-timeout"
title: "L01 encoding and workspace checks finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:46:04Z"
updated_at: "2026-09-04T20:13:21Z"
claimed-by: dcc0dcc6-a069-404e-9834-e7c9b9d9d9cd
---

# L01 encoding and workspace checks finish

## Why

Review of [[s-l01]] left ROADMAP L01 unfinished: O1 (`cargo test -p draconic-conformance --test encoding`) timed out at 120s without matching `test result: ok.`, and O2 `cargo test --workspace` also timed out at 120s. The stdlib location still needs the L01 Loop to leave the encoding fixtures and the workspace green, not a hung encoding or workspace check.

## Done

`cargo test -p draconic-conformance --test encoding` and `cargo test --workspace` each finish with `test result: ok.` after the ROADMAP L01 work. A Program can convert UTF-8 bytes↔string, Base64, and hex through the designed encoding surface, and invalid input errors rather than silently corrupting. Tests under `tests/conformance` fixtures `stdlib/encoding` lock that combined surface. If the hang comes from the L01 encoding surface, fix that surface so both checks hold. Mark L01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l01]]**: that slice stays sealed `failed`
- **L01.01**: UTF-8 encode/decode: string ↔ bytes; invalid UTF-8 error (already `done`)
- **L01.02**: Base64 encode/decode (already `done`)
- **L01.03**: Hex encode/decode (already `done`)
- **L02**: Collections helpers
- **L03**: Crypto SHA-256 / CSPRNG
- **L08**: URL / query parse + serialize
- Re-filing E15 JSON or RegExp as encoding work

## Oracle checklist

- [x] O1: L01 UTF-8, Base64, and hex fixtures are locked by the stdlib encoding conformance tests
  CHECK: cargo test -p draconic-conformance --offline --test encoding
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=53dc6766d0b21344 bytes=3306 at=2026-09-04T20:12:42.738Z

- [x] O2: workspace tests finish after the L01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --offline --test encoding
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d6015d2546765868 bytes=94875 at=2026-09-04T20:13:10.702Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l01-oracle-timeout]]`

## See also

ROADMAP.md L01, `tests/conformance/tests/encoding.rs`, `tests/conformance/fixtures/stdlib/encoding`, `crates/draconic-backend-llvm/src/es_encoding.rs`, CONTEXT.md, [[stdlib]], [[s-l01]], [[ticket-179-l01-oracle-timeout]].
