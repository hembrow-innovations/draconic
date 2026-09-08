---
id: "ticket-695-parser-rust-dev-misses"
title: "Parser lib.rs far over file budget"
kind: ticket
status: promoted
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Parser lib.rs far over file budget

## Signal

Rust-development review of crates/draconic-parser. src/lib.rs is 10483 lines (hard miss). Impl through about 7465, tests from 7466.

## Fit

Promoted to [[slice-706-parser-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

fuzz.rs and fuzz harness files are in budget. No other rule misses reported.

## Parent

[[slice-706-parser-file-budget]]
