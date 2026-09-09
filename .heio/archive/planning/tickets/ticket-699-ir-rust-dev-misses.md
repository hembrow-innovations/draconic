---
id: "ticket-699-ir-rust-dev-misses"
title: "IR lib.rs over budget and dump_module is pub"
kind: ticket
status: closed
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# IR lib.rs over budget and dump_module is pub

## Signal

Rust-development review of crates/draconic-ir. lib.rs is 8474 lines (hard miss). dump_module around 6763 is pub but only used by in-crate tests.

## Fit

Promoted to [[slice-710-ir-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

One shared IR, no emit, no process-global lower state.

## Parent

[[slice-710-ir-file-budget]]
