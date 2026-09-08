---
id: "ticket-697-linker-rust-dev-misses"
title: "Linker lib.rs over budget and Diagnostics lack codes"
kind: ticket
status: promoted
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# Linker lib.rs over budget and Diagnostics lack codes

## Signal

Rust-development review of crates/draconic-linker. lib.rs is 6004 lines (hard miss). About 40 Diagnostic::new sites have no codes::* / with_code.

## Fit

Promoted to [[slice-708-linker-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

ACTIVE_PACKAGES thread-local is allowed. Result uses Diagnostic. One .rs file only.

## Parent

[[slice-708-linker-file-budget]]
