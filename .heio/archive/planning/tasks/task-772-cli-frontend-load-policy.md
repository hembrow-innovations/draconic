---
id: "task-772-cli-frontend-load-policy"
title: "CLI reuses Frontend parse policy"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "rust-dev-audit"
slice: "slice-715-cli-file-budget"
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T18:00:00Z"
---

# CLI reuses Frontend parse policy

## Blocked by

None. Prefactor for [[task-742-cli-split-main]].

## Done

format_source and repl_buffer_status do not reimplement Script-then-Module parse. They call Frontend (or one CLI helper that calls Frontend) for that policy. parse command may still call parse_and_dump. cargo test -p draconic-cli is green. Still no clap.

## Context

frontend lib.rs owns Script vs Module. CLI format_source at main.rs:415 copies it without link. repl_buffer_status does it again. Prefactor so splitting main.rs does not copy the policy into more files.

## Verify

cargo test -p draconic-cli --offline. format_source no longer calls parse/parse_module directly.

scope: crates/draconic-cli/src/main.rs, crates/draconic-frontend/src/lib.rs only if a tiny pub helper is required

## Links

[[slice-715-cli-file-budget]] crates/draconic-frontend/src/lib.rs

## Agent Brief

**Category:** layout
**Summary:** CLI fmt/repl stop copying Frontend load policy.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: one Script vs Module policy
- Contract-first: do not invent language behaviour

**Current behavior:**
format_source and repl re-parse Script then Module themselves.

**Desired behavior:**
Same fmt/repl behaviour via Frontend policy. No clap.

**Key interfaces:**
- compile_source / check_source / parse helpers on Frontend
- size-file-budget: main.rs must not grow; extracting a small fmt module ≤1000 is allowed

**Acceptance criteria:**
- [x] format_source does not call draconic_parser::parse directly
- [x] repl completeness uses the same policy helper
- [x] cargo test -p draconic-cli
- [x] no clap

**Out of scope:**
- Splitting the rest of main.rs (task-742)
- Rewiring parser to check as a new pipeline
- Language behaviour or ROADMAP rows

**Explain this part:**
cmd_parse may still dump AST via parser. Do not link modules during fmt.

## Gauntlet

- **round 1**: `cargo test -p draconic-cli --offline` — win. format_source and repl_buffer_status call Frontend `parse_source`; no clap.
