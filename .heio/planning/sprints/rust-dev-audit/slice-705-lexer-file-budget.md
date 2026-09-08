---
id: "slice-705-lexer-file-budget"
title: "Lexer file budget and regexp Diagnostic"
kind: slice
status: frozen
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Lexer file budget and regexp Diagnostic

## Why

Lexer tokenize stays stage 1, but lib.rs is one 2926-line bag and regexp validation talks String instead of Diagnostic.

## Done

Every crates/draconic-lexer .rs file is ≤1000 lines. validate_regexp_literal and validate_regexp_flags return Result<(), Diagnostic>. cargo test -p draconic-lexer is green.

## Blocked by

None.

## Non-goals

- **parser/check changes**: lexer only
- **ROADMAP atom**
- **moving unit tests out of the owning .rs file**

## Oracle checklist


- [ ] O1: lexer rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-lexer"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: pending
- [ ] O2: lexer tests green
  CHECK: cargo test -p draconic-lexer --offline
  EXPECT: test result: ok.
  EVIDENCE: pending
- [ ] O3: regexp helpers return Diagnostic
  CHECK: rg -n "fn validate_regexp_literal|fn validate_regexp_flags" crates/draconic-lexer/src -A 1
  EXPECT: Result<(), Diagnostic>
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- [[task-716-lexer-split-lib]]
- [[task-717-lexer-regexp-diagnostic]]

## See also

[[ticket-694-lexer-rust-dev-misses]], size-file-budget, test-same-file, crates/draconic-lexer
