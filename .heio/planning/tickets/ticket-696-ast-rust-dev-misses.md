---
id: "ticket-696-ast-rust-dev-misses"
title: "AST lib.rs and print.rs over file budget"
kind: ticket
status: promoted
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# AST lib.rs and print.rs over file budget

## Signal

Rust-development review of crates/draconic-ast. lib.rs is 2462 lines. print.rs is 1703 lines. Both over the 1250 hard cap.

## Fit

Promoted to [[slice-707-ast-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

Boxing, no second IR, workspace deps, and same-file tests hold.

## Parent

[[slice-707-ast-file-budget]]
