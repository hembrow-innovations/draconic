---
id: "slice-711-backend-js-file-budget"
title: "JS backend es_* split and pointer codes"
kind: slice
status: met
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:30:00Z"
---

# JS backend es_* split and pointer codes

## Why

JS emit has no es_* split. lib.rs 1911 and emit.rs 1373 over 1250. Pointer native_only_diag lacks with_code.

## Done

Every crates/draconic-backend-js .rs file is ≤1000. Pointer native-only failures use codes::*. cargo test -p draconic-backend-js is green. Native-only features still hard-error, not silent JS.

## Blocked by

None.

## Non-goals

- **silent JS emit of native-only features**
- **inkwell**

## Oracle checklist


- [x] O1: js backend rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-backend-js"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: max-loc-ok
- [x] O2: js backend tests green
  CHECK: cargo test -p draconic-backend-js --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 64 passed
- [x] O3: pointer native_only uses with_code
  CHECK: rg -n "native_only_diag|with_code" crates/draconic-backend-js/src
  EXPECT: with_code
  EVIDENCE: crates/draconic-backend-js/src/native.rs:15 .with_code(codes::POINTER_UNSUPPORTED)

## Pool

Durable links to task ids. Never drop them.

- [[task-733-js-split-emit]]
- [[task-734-js-pointer-codes]]

## See also

[[ticket-700-backend-js-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-backend-js
