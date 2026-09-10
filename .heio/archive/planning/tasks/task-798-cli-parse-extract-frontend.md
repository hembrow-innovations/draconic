---
id: "task-798-cli-parse-extract-frontend"
title: "Parse and extract use Frontend parse_source"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-787-frontend-cli-seam"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T18:20:00Z"
---

# Parse and extract use Frontend parse_source

## Blocked by

None.

## Done

`cmd_parse` does not call `parse_and_dump`. Extract uses Frontend parse policy (Script-then-Module), not Module-only `parse_module`. Slice O1 and O2 hold. CLI parse and extract tests stay green.

## Context

Frontend `parse_source` already retries Module after Script. Parse CLI and extract should not copy that. Dump formatting can stay in the parser crate if `parse_source` then print is enough. Do not change Script vs Module detection rules.

## Verify

Slice O1 and O2.

scope: crates/draconic-cli/src/cmd/cmd_parse.rs, crates/draconic-cli/src/extract.rs, Frontend only if a tiny parse+dump helper is required

## Links

[[slice-787-frontend-cli-seam]] [[ticket-780-frontend-cli-bypass]]

## Agent Brief

**Category:** layout
**Summary:** Parse and extract go through Frontend parse_source.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none
- Purpose: one parse policy
- Contract-first: do not invent a third goal

**Current behavior:**
parse CLI uses parser dump. extract uses parse_module only.

**Desired behavior:**
Frontend parse_source. CLI tests green.

**Key interfaces:**
- draconic_frontend::parse_source
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] slice O1 frontend
- [x] cargo test -p draconic-cli
- [x] extract still emits the same JSON shape

**Out of scope:**
- REPL inspect (that is [[task-799-repl-emit-in-backend]])
- test262 (that is [[task-800-test262-frontend-link]])
- bindgen

**Explain this part:**
Module-only extract skips Script-first policy. Keep dump text stable unless a test forces a change; then update the test.
