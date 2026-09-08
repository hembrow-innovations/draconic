---
id: "slice-708-linker-file-budget"
title: "Linker split and diagnostic codes"
kind: slice
status: met
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:45:00Z"
---

# Linker split and diagnostic codes

## Why

Linker is one 6004-line file and about 40 Diagnostic::new sites omit codes::*.

## Done

Every crates/draconic-linker .rs file is ≤1000. Compiler-path Diagnostic::new sites use with_code / codes::*. cargo test -p draconic-linker is green.

## Blocked by

None.

## Non-goals

- **owning check or emit**
- **removing ACTIVE_PACKAGES package context**

## Oracle checklist


- [x] O1: linker rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-linker"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: max-loc-ok
- [x] O2: linker tests green
  CHECK: cargo test -p draconic-linker --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 31 passed
- [x] O3: linker diagnostics carry codes
  CHECK: python3 -c 'from pathlib import Path; import re; n=0; c=0
for p in Path("crates/draconic-linker").rglob("*.rs"):
 s=p.read_text(); n+=len(re.findall(r"Diagnostic::new", s)); c+=len(re.findall(r"with_code", s))
print("linker-codes-ok" if c>=n and n>0 else "new=%d codes=%d" % (n,c))'
  EXPECT: linker-codes-ok
  EVIDENCE: linker-codes-ok

## Pool

Durable links to task ids. Never drop them.

- [[task-724-linker-split-lib]]
- [[task-725-linker-err-codes]]

## See also

[[ticket-697-linker-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-linker
