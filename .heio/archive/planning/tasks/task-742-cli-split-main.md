---
id: "task-742-cli-split-main"
title: "Split cli main.rs"
kind: task
status: completed
mode: afk
blocked_by: ["task-772-cli-frontend-load-policy"]
sprint: "rust-dev-audit"
slice: "slice-715-cli-file-budget"
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T22:00:00Z"
---

# Split cli main.rs

## Blocked by

[[task-772-cli-frontend-load-policy]]: one Frontend load policy before splitting main.rs.

## Done

main.rs ≤1000. Subcommand dispatch lives in cmd_*.rs files each ≤1000. Unit tests in main.rs move with the seam or stay if main.rs remains ≤1000.

## Context

main.rs 1768, tests from 1589. Hand argv match stays. No clap.

## Verify

cargo test -p draconic-cli. main.rs ≤1000.

scope: crates/draconic-cli/src/main.rs and new src/cmd_*.rs or similar except extract.rs and c_header.rs

## Links

[[slice-715-cli-file-budget]]

## Agent Brief

**Category:** layout
**Summary:** Split CLI main.rs under 1000.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none unless this task names a Diagnostic change
- Purpose: rust-development file budget and named rule misses
- Contract-first: do not invent language behaviour

**Current behavior:**
main.rs 1768 lines.

**Desired behavior:**
main.rs ≤1000, no clap.

**Key interfaces:**
- `mod foo;` plus `foo.rs` (arch-file-modules)
- same-file `#[cfg(test)]` moves with the seam (test-same-file)
- size-file-budget: target ≤1000 LOC, hard cap 1250

**Acceptance criteria:**
- [x] main.rs ≤1000
- [x] no clap dependency
- [x] cargo test -p draconic-cli

**Out of scope:**
- Language behaviour or ROADMAP rows
- Extracting unit tests into crate-level tests/ (test-same-file)
- cargo test --workspace as this task's oracle
- New crates or workspace members
- Editing extract.rs or c_header.rs

**Explain this part:**
Keep compile_path / check_path. unsafe isatty only.

## Gauntlet

- **round 1**: `cargo test -p draconic-cli --offline` — win. main.rs 274; no clap; test result ok.
