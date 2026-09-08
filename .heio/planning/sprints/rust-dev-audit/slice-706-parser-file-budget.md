---
id: "slice-706-parser-file-budget"
title: "Parser lib.rs feature split"
kind: slice
status: frozen
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Parser lib.rs feature split

## Why

Parser is stage 2 but src/lib.rs is 10483 lines, the size-file-budget incorrect example.

## Done

Every crates/draconic-parser .rs file is ≤1000 lines. parse and parse_module stay the public entry. cargo test -p draconic-parser is green.

## Blocked by

None.

## Non-goals

- **changing parse grammar**
- **ROADMAP atom**
- **crate-level unit tests/**

## Oracle checklist


- [ ] O1: parser rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-parser"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: pending
- [ ] O2: parser tests green
  CHECK: cargo test -p draconic-parser --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- [[task-770-parser-context]]
- [[task-718-parser-extract-stmt]]
- [[task-719-parser-extract-expr]]
- [[task-720-parser-extract-module]]
- [[task-721-parser-finish-budget]]

## See also

[[ticket-695-parser-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-parser/src/lib.rs
