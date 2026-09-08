---
id: "slice-707-ast-file-budget"
title: "AST types and printer file budget"
kind: slice
status: frozen
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# AST types and printer file budget

## Why

Shared AST is two oversized files: types/dump in lib.rs (2462) and print.rs (1703).

## Done

Every crates/draconic-ast .rs file is ≤1000. dump_program and print_program stay the public printers. cargo test -p draconic-ast is green.

## Blocked by

None.

## Non-goals

- **a second IR**
- **check or emit logic in ast**
- **unboxing recursive nodes**

## Oracle checklist


- [ ] O1: ast rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-ast"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: pending
- [ ] O2: ast tests green
  CHECK: cargo test -p draconic-ast --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- [[task-722-ast-split-lib]]
- [[task-723-ast-split-print]]

## See also

[[ticket-696-ast-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-ast
