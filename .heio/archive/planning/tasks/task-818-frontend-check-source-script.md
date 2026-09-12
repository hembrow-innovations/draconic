---
id: "task-818-frontend-check-source-script"
title: "Frontend check_source stays Script"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-817-frontend-check-source-script"
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T16:30:00Z"
---

# Frontend check_source stays Script

## Blocked by

None.

## Done

Frontend tests lock Script-only `check_source` / `compile_source` and Module-only `*_module` string APIs. `parse_source` may still retry Module.

## Context

Audit noted that `parse_source` retries Module while `check_source` does not. [[architecture-frontend]] and [[architecture-pipeline]] already require string check/compile to be Script. This sitting locks that contract. Do not retry Module on `check_source`.

## Verify

Slice O1.

scope: draconic-frontend tests and docs-facing comments on the string APIs; no Linker change

## Links

[[slice-817-frontend-check-source-script]] [[ticket-808-frontend-check-source-script-only]] [[architecture-frontend]] [[architecture-pipeline]]

## Agent Brief

**Category:** test/lock
**Summary:** Lock Frontend string check/compile as Script-only; do not retry Module.

**Intent (required when product behaviour changes):**
- **No product behaviour change**
- Promise ids: none
- Purpose: [[architecture-frontend]] [[architecture-pipeline]] — `check_source` / `compile_source` are Script; Module strings use `*_module`
- Contract-first: assert existing architecture; do not invent Module retry on string check

**Current behavior:**
`parse_source` tries Script parse then Module parse (fmt and dump). `check_source` and `compile_source` parse Script only. `check_source_module` / `compile_source_module` are the Module string APIs. Path load still Script-first then Module. Export-only source can parse via `parse_source` and fail `check_source`.

**Desired behavior:**
Keep that split. Add Frontend tests that:
- `check_source` and `compile_source` diagnostic on `export let x = 1;`
- `check_source_module` and `compile_source_module` accept that buffer
- `parse_source` still accepts that buffer (existing retry test may already cover this)
- Script `let x = 1;` still succeeds on `check_source` / `compile_source`
Do not make `check_source` call the Module retry. Do not change `load_program` / `compile_path`. Do not edit ROADMAP.

**Key interfaces:**
- `check_source` / `compile_source` (Script)
- `check_source_module` / `compile_source_module` (Module, no link graph)
- `parse_source` (Script then Module retry for tools)
- size-file-budget: frontend lib stays under 1000 if already under; do not push over 1250

**Acceptance criteria:**
- [x] `cargo test -p draconic-frontend --offline` prints `test result: ok.`
- [ ] Tests exist that export-only source fails Script string check/compile and succeeds Module string check/compile
- [x] `check_source` still does not retry Module
- [x] Path Script-then-Module policy unchanged

**Out of scope:**
- making `check_source` retry Module
- Linker / `compile_path` / `load_program`
- LLVM walker / host catalog
- marking E17.02 or E18.44 done

## Gauntlet

- **round**: 1
- **command**: cargo test -p draconic-frontend --offline
- **result**: win
- **gap**: none. Script string check/compile stay Script-only. Export-only cannot succeed on `*_module` without a link graph (checker: import/export must be linked). TLA is the Module-goal probe; path load is unchanged.
