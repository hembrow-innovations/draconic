---
id: "task-759-llvm-default-walker"
title: "Default LLVM path is one IR walker"
kind: task
status: ready
mode: afk
blocked_by: ["task-758-llvm-shared-emitter"]
sprint: "rust-dev-audit"
slice: "slice-756-llvm-one-walker"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T17:30:00Z"
---

# Default LLVM path is one IR walker

## Blocked by

[[task-758-llvm-shared-emitter]]: share SlotTy/Emitter first.

## Done

Programs that miss every is_* adapter are lowered by the shared walker (es_expr-shaped) instead of unsupported_native_diagnostic when the walker can emit them. Unsupported nodes still diagnostic. emit_llvm_ir unchanged.

## Context

Today emit_llvm_ir_raw last-matches is_es_expr_module then empty then hard-error. The walker must become the default for leftover IR, matching JS emit.rs: one match over Stmt/Expr. Keep the existing is_* arms so current fixtures still classify. Do not add a new is_*_module.

## Verify

cargo test -p draconic-backend-llvm --offline. A program of supported expr/control-flow that is not an is_* fixture still emits LLVM (add a same-file test through emit_llvm_ir if none exists).

scope: crates/draconic-backend-llvm/src/lib.rs emit_llvm_ir_raw, crates/draconic-backend-llvm/src/es_expr.rs, shared emitter module

## Links

[[slice-756-llvm-one-walker]]

## Agent Brief

**Category:** layout
**Summary:** Fall through to one IR walker instead of unsupported.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: LLVM consumes IR nodes, not whole-program shapes
- Contract-first: do not invent language behaviour

**Current behavior:**
Unknown program shapes hard-error even when es_expr could lower the nodes.

**Desired behavior:**
Default path walks Stmt/Expr. Existing adapter arms still win first. Native observations unchanged for current tests.

**Key interfaces:**
- emit_llvm_ir / emit_llvm_ir_raw
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [ ] leftover supported expr programs emit via the walker
- [ ] still-unsupported nodes diagnostic
- [ ] no new is_*_module function
- [ ] cargo test -p draconic-backend-llvm

**Out of scope:**
- Deleting host_* or fixture printers
- inkwell/llvm-sys
- Language behaviour or ROADMAP rows

**Explain this part:**
Do not steal programs from earlier is_* arms. Order stays load-bearing until contract-dispatch.
