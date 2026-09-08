---
id: "slice-710-ir-file-budget"
title: "IR split and dump_module visibility"
kind: slice
status: frozen
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# IR split and dump_module visibility

## Why

The one shared IR lives in an 8474-line lib.rs and dump_module is pub though only tests use it.

## Done

Every crates/draconic-ir .rs file is ≤1000. dump_module is pub(crate). lower stays the public lowering entry. cargo test -p draconic-ir is green.

## Blocked by

None.

## Non-goals

- **a second IR**
- **JS or LLVM emit in this crate**

## Oracle checklist


- [ ] O1: ir rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-ir"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: pending
- [ ] O2: ir tests green
  CHECK: cargo test -p draconic-ir --offline
  EXPECT: test result: ok.
  EVIDENCE: pending
- [ ] O3: dump_module is pub(crate)
  CHECK: rg -n "fn dump_module" crates/draconic-ir/src
  EXPECT: pub(crate) fn dump_module
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- [[task-730-ir-dump-pub-crate]]
- [[task-731-ir-extract-types]]
- [[task-732-ir-split-lower]]

## See also

[[ticket-699-ir-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-ir
