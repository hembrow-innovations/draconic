---
id: "task-769-frontend-compile-target"
title: "Frontend passes CompileTarget"
kind: task
status: ready
mode: afk
blocked_by: ["task-765-host-catalog-sync"]
sprint: "rust-dev-audit"
slice: "slice-757-host-catalog"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Frontend passes CompileTarget

## Blocked by

[[task-765-host-catalog-sync]]: catalog exists.

## Done

check_path / compile_path (or a clear sibling) can pass CompileTarget into check_for_target / check_module_for_target. CLI check/build on js uses it so JS emit is not the only host-policy adapter. Default without a target keeps today's behaviour. cargo test -p draconic-frontend and cargo test -p draconic-cli are green.

## Context

Checker already has check_for_target. Frontend never calls it. JS emit then consults host_api_unsupported_diagnostic. Put policy at the Frontend seam.

## Verify

cargo test -p draconic-frontend --offline. cargo test -p draconic-cli --offline. A frontend test shows native-only host use diagnostics when target is js.

scope: crates/draconic-frontend/src/lib.rs, crates/draconic-cli/src/main.rs cmd_check/cmd_build only as needed to pass target, crates/draconic-backend-js/src/lib.rs only to stop duplicating a check the frontend already did if tests allow

## Links

[[slice-757-host-catalog]] crates/draconic-frontend/src/lib.rs

## Agent Brief

**Category:** layout
**Summary:** Host-target policy lives at Frontend, not only JS emit.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: check_for_target is the build seam
- Contract-first: do not invent language behaviour

**Current behavior:**
compile_path always check() with no target. JS emit rejects native-only host names.

**Desired behavior:**
Frontend can check for js vs native. Untargeted compile matches today.

**Key interfaces:**
- compile_path / check_path / check_for_target
- CompileTarget
- size-file-budget: frontend lib.rs stays small

**Acceptance criteria:**
- [ ] Frontend can pass CompileTarget
- [ ] untargeted path behaviour unchanged
- [ ] cargo test -p draconic-frontend
- [ ] cargo test -p draconic-cli

**Out of scope:**
- New host APIs
- LLVM walker
- clap
- Language behaviour or ROADMAP rows

**Explain this part:**
Do not reassemble parser/check/ir in the CLI. Do not make check depend on runtime.
