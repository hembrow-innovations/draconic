---
id: "slice-713-pkg-file-budget"
title: "Pkg lib cache resolve file budget"
kind: slice
status: met
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:50:00Z"
---

# Pkg lib cache resolve file budget

## Why

Packages already split files, but lib.rs 1856, cache.rs 1277, and resolve.rs 1121 still miss the budget.

## Done

Every crates/draconic-pkg .rs file is ≤1000. Hand-written Error enums stay. No serde toml. cargo test -p draconic-pkg is green.

## Blocked by

None.

## Non-goals

- **serde-for-toml**
- **Diagnostic as pkg Error**

## Oracle checklist


- [x] O1: pkg rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-pkg"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: max-loc-ok after task-735, task-736, task-737
- [x] O2: pkg tests green
  CHECK: cargo test -p draconic-pkg --offline
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-pkg --offline: 313 passed

## Pool

Durable links to task ids. Never drop them.

- [[task-735-pkg-split-lib]]
- [[task-736-pkg-split-cache]]
- [[task-737-pkg-split-resolve]]

## See also

[[ticket-702-pkg-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-pkg
