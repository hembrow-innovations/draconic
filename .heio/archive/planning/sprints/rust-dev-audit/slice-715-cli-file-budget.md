---
id: "slice-715-cli-file-budget"
title: "CLI main extract header and test split"
kind: slice
status: met
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:55:00Z"
---

# CLI main extract header and test split

## Why

CLI already uses frontend and hand argv, but main.rs 1768, tests/extract.rs 2343, extract.rs 1141, c_header.rs 1021, and tests/build.rs 1227 miss the budget.

## Done

Every crates/draconic-cli .rs file is ≤1000 including tests/. cargo test -p draconic-cli is green. Still no clap. compile_path / check_path stay the compiler entry.

## Blocked by

None.

## Non-goals

- **introducing clap**
- **rewiring parser to check**

## Oracle checklist


- [x] O1: cli rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-cli"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: max-loc-ok after task-772, task-742, task-743, task-744, task-745
- [x] O2: cli tests green
  CHECK: cargo test -p draconic-cli --offline
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-cli --offline: test result: ok. on lib, bin, and every tests/*.rs binary including extract, extract_calls, extract_class

## Pool

Durable links to task ids. Never drop them.

- [[task-772-cli-frontend-load-policy]]
- [[task-742-cli-split-main]]
- [[task-743-cli-split-extract]]
- [[task-744-cli-split-c-header]]
- [[task-745-cli-split-tests-build]]

## See also

[[ticket-704-cli-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-cli
