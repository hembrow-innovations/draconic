---
id: "slice-714-runtime-file-budget"
title: "Runtime file budget and sibling modules"
kind: slice
status: frozen
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Runtime file budget and sibling modules

## Why

Runtime policy holds, but lib.rs 3897, abi.rs 3270, and host_abi_tests.rs 5143 are over cap, and crypto/testing/url are inline mods.

## Done

Every crates/draconic-runtime .rs file is ≤1000. crypto.rs, testing.rs, and url.rs exist as sibling file modules. cargo test -p draconic-runtime is green. C sources stay exempt.

## Blocked by

None.

## Non-goals

- **deny-by-default host I/O**
- **folding abort into HostError**
- **counting C files toward the Rust budget**

## Oracle checklist


- [ ] O1: runtime rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-runtime"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: pending
- [ ] O2: runtime tests green
  CHECK: cargo test -p draconic-runtime --offline
  EXPECT: test result: ok.
  EVIDENCE: pending
- [ ] O3: crypto testing url are file modules
  CHECK: rg -n "^pub mod crypto|^mod crypto|^pub mod testing|^pub mod url" crates/draconic-runtime/src/lib.rs
  EXPECT: mod crypto
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- [[task-738-runtime-extract-inline-mods]]
- [[task-739-runtime-split-lib]]
- [[task-740-runtime-split-abi]]
- [[task-741-runtime-split-host-abi-tests]]

## See also

[[ticket-703-runtime-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-runtime
