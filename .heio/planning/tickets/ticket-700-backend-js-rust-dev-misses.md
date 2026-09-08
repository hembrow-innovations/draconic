---
id: "ticket-700-backend-js-rust-dev-misses"
title: "JS backend files over budget and pointer diags lack codes"
kind: ticket
status: promoted
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T16:00:00Z"
---

# JS backend files over budget and pointer diags lack codes

## Signal

Rust-development review of crates/draconic-backend-js. lib.rs is 1911 lines. emit.rs is 1373 lines. native_only_diag pointer failures use Diagnostic::new with no with_code. No es_* split yet.

## Fit

Promoted to [[slice-711-backend-js-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

source_map.rs is 432. dual-js-policy and ir-shared hold.

## Parent

[[slice-711-backend-js-file-budget]]
