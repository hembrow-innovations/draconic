---
id: "task-799-repl-emit-in-backend"
title: "REPL last-expression print lives in JS backend"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-787-frontend-cli-seam"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-10T05:30:00Z"
---

# REPL last-expression print lives in JS backend

## Blocked by

None.

## Done

CLI REPL does not clone IR and splice Node `util.inspect`. JS backend exposes a small `emit_js` variant for REPL print. Existing repl tests stay green.

## Context

`cmd_repl.rs` currently compiles, pops the last expr, emits twice, splices inspect. That is emit policy. Keep Frontend `compile_source` for the compile seam. Do not change REPL session binding behavior.

## Verify

`cargo test -p draconic-cli --test repl --offline` ok. `cargo test -p draconic-backend-js --offline` ok.

scope: crates/draconic-backend-js/src/lib.rs, crates/draconic-cli/src/cmd/cmd_repl.rs

## Links

[[slice-787-frontend-cli-seam]] [[ticket-780-frontend-cli-bypass]]

## Agent Brief

**Category:** layout
**Summary:** Move REPL print splicing into the JS backend.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: REPL still prints the last expression
- Contract-first: do not invent a new REPL language

**Current behavior:**
CLI clones Module and splices util.inspect.

**Desired behavior:**
Backend emit helper. repl tests green.

**Key interfaces:**
- emit_js
- compile_source
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [ ] cargo test -p draconic-cli --test repl
- [ ] cargo test -p draconic-backend-js
- [ ] cmd_repl does not splice util.inspect

**Out of scope:**
- native REPL
- extract
- test262

**Explain this part:**
The CLI should not know inspect. Observable REPL output stays the same.
