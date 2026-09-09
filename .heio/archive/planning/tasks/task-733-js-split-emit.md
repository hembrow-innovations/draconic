---
id: "task-733-js-split-emit"
title: "Split JS backend lib.rs and emit.rs"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-711-backend-js-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T21:30:00Z"
---

# Split JS backend lib.rs and emit.rs

## Blocked by

None.

## Done

Every crates/draconic-backend-js .rs file is ≤1000. Prefer es_* names like LLVM.

## Context

lib.rs 1911 (prod about 1345 then tests). emit.rs 1373 no tests. source_map.rs 432 stays. Split by emit feature seam.

## Verify

Slice O1 and O2.

scope: crates/draconic-backend-js/src/**

## Links

[[slice-711-backend-js-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split JS emit files under 1000 using es_* seams.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
Two files over 1250 and no es_* modules.

**Desired behavior:**
es_* / existing modules ≤1000. emit_js still public.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] Slice O1 max-loc-ok
- [x] cargo test -p draconic-backend-js
- [x] dual-js-policy still hard-errors pointers/extern/host

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members

**Explain this part:**
Do not invent a second IR. Consume draconic_ir::Module.

## Gauntlet

- **round 1**: python3 max-loc on crates/draconic-backend-js; cargo test -p draconic-backend-js --offline — **win**. max-loc-ok. test result: ok. 64 passed. emit_js still public. native.rs still reject_native_only / reject_extern_ffi / reject_host_api_name.
