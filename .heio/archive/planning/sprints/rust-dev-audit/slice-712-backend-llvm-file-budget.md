---
id: "slice-712-backend-llvm-file-budget"
title: "LLVM backend file budget and extra globals"
kind: slice
status: met
sprint: "rust-dev-audit"
blocked_by: ["slice-756-llvm-one-walker"]
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:30:00Z"
---

# LLVM backend file budget and extra globals

## Why

LLVM already splits es_* / host_* but 37 files still over 1000 and extra thread-locals/globals exist beyond allowed JS interp this/new.target.

## Done

Every crates/draconic-backend-llvm .rs file is ≤1000. REGEXP_STATICS, FN_REG, SINK_LINES are not leftover extra thread-locals. TOOLS OnceLock and AtomicU64 object id counters are not process-global. CURRENT_THIS and CURRENT_NEW_TARGET may remain. cargo test -p draconic-backend-llvm is green.

## Blocked by

[[slice-756-llvm-one-walker]]: collapse whole-program adapters before splitting leftover files.

## Non-goals

- **inkwell or llvm-sys**
- **removing allowed CURRENT_THIS / CURRENT_NEW_TARGET**

## Oracle checklist


- [x] O1: llvm backend rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-backend-llvm"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: max-loc-ok
- [x] O2: llvm backend tests green
  CHECK: cargo test -p draconic-backend-llvm --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 314 passed

## Pool

Durable links to task ids. Never drop them.

- [[task-746-llvm-split-lib]]
- [[task-747-llvm-split-es-builtins]]
- [[task-748-llvm-split-es-classes]]
- [[task-749-llvm-split-es-expr]]
- [[task-750-llvm-split-es-functions]]
- [[task-751-llvm-split-over-2000]]
- [[task-752-llvm-split-host-over-1250]]
- [[task-753-llvm-split-es-over-1250]]
- [[task-754-llvm-split-over-1000]]
- [[task-755-llvm-no-extra-globals]]

## See also

[[ticket-701-backend-llvm-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-backend-llvm
