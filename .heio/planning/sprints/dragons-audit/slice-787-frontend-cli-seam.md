---
id: "slice-787-frontend-cli-seam"
title: "CLI and test262 use Frontend"
kind: slice
status: frozen
sprint: "dragons-audit"
blocked_by: []
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-10T05:30:00Z"
---

# CLI and test262 use Frontend

## Why

Frontend already owns Script vs Module then check then lower. Parse, extract, REPL print, and test262 force-link still assemble stages.

## Done

`cmd_parse` and extract call Frontend `parse_source` (or a Frontend path helper that owns the same policy). REPL last-expression print lives in the JS backend. test262 force-link goes through one Frontend function. CLI and test262 tests stay green.

## Blocked by

None.

## Non-goals

- **bindgen C parser**
- **doc comment scanner**
- **changing Script vs Module policy**

## Oracle checklist

- [ ] O1: parse command uses Frontend
  CHECK: python3 -c 'from pathlib import Path; t=Path("crates/draconic-cli/src/cmd/cmd_parse.rs").read_text(); print("frontend" if "draconic_frontend" in t and "parse_and_dump" not in t else "rewire")'
  EXPECT: frontend
  EVIDENCE: pending
- [ ] O2: cli tests green
  CHECK: cargo test -p draconic-cli --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- [[task-798-cli-parse-extract-frontend]]
- [[task-799-repl-emit-in-backend]]
- [[task-800-test262-frontend-link]]

## See also

[[ticket-780-frontend-cli-bypass]], crates/draconic-frontend/src/lib.rs, crates/draconic-cli/src/extract.rs, crates/draconic-cli/src/cmd/cmd_repl.rs, tests/test262/src/lib.rs
