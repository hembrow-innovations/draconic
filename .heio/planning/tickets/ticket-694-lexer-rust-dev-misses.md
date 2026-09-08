---
id: "ticket-694-lexer-rust-dev-misses"
title: "Lexer over file budget and String regexp errors"
kind: ticket
status: promoted
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Lexer over file budget and String regexp errors

## Signal

Rust-development review of crates/draconic-lexer. lib.rs is 2926 lines (hard miss over 1250). validate_regexp_literal and validate_regexp_flags return Result<(), String> instead of Diagnostic.

## Fit

Promoted to [[slice-705-lexer-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

regexp.rs is 272 lines and in budget. Edition, workspace deps, and same-file tests otherwise hold.

## Parent

[[slice-705-lexer-file-budget]]
