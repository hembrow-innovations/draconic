---
id: "slice-709-check-file-budget"
title: "Checker Binder Checker and host_api split"
kind: slice
status: met
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:30:00Z"
---

# Checker Binder Checker and host_api split

## Why

Check is the size-file-budget incorrect example: lib.rs 9342 and host_api.rs 1775.

## Done

Every crates/draconic-check .rs file is ≤1000. check / check_module / check_for_target stay public. cargo test -p draconic-check is green.

## Blocked by

None.

## Non-goals

- **JS or LLVM emit**
- **a second IR**

## Oracle checklist


- [x] O1: check rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-check"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: max-loc-ok (lib.rs 121; no check .rs over 1000)
- [x] O2: check tests green
  CHECK: cargo test -p draconic-check --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 278 passed; 0 failed

## Pool

Durable links to task ids. Never drop them.

- [[task-771-check-bind-in-check]]
- [[task-726-check-split-host-api]]
- [[task-727-check-extract-binder]]
- [[task-728-check-extract-checker]]
- [[task-729-check-finish-lib]]

## See also

[[ticket-698-check-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-check
