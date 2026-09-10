---
id: "task-800-test262-frontend-link"
title: "test262 force-link goes through Frontend"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-787-frontend-cli-seam"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T00:30:00Z"
---

# test262 force-link goes through Frontend

## Blocked by

None.

## Done

test262 no longer calls `link_entry` then `check_module` then `lower` itself. One Frontend function owns that path (including `import defer` if that is why force-link exists). test262 crate tests stay green.

## Context

Frontend comment says callers should not re-assemble parser, check, and IR. test262 force-link is the remaining production-shaped bypass. Prefer adding `compile_path` / `check_path` capability over copying stages. Do not expand the Test262 allowlist in this sitting.

## Verify

`cargo test -p draconic-test262 --offline` ok.

scope: tests/test262/src/lib.rs, crates/draconic-frontend/src/lib.rs

## Links

[[slice-787-frontend-cli-seam]] [[ticket-780-frontend-cli-bypass]]

## Agent Brief

**Category:** layout
**Summary:** test262 compiles through Frontend.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: one compile seam
- Contract-first: do not invent new Test262 host hooks

**Current behavior:**
test262 wires link, check_module, lower.

**Desired behavior:**
Frontend function. test262 tests green.

**Key interfaces:**
- draconic_frontend compile/check path
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] cargo test -p draconic-test262
- [x] no link_entry+check_module+lower in test262 src
- [x] allowlist unchanged

**Out of scope:**
- CLI parse/extract
- REPL
- new Test262 files

**Explain this part:**
import defer is why force-link exists. Put that policy on Frontend, do not keep a private pipeline.
