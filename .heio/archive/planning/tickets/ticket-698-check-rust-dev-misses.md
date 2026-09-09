---
id: "ticket-698-check-rust-dev-misses"
title: "Checker lib.rs and host_api.rs over file budget"
kind: ticket
status: closed
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# Checker lib.rs and host_api.rs over file budget

## Signal

Rust-development review of crates/draconic-check. lib.rs is 9342 lines. host_api.rs is 1775 lines. Both over 1250. Binder around 1125 and Checker around 2911 were never extracted.

## Fit

Promoted to [[slice-709-check-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

Diagnostic plus codes on hard paths hold. No emit leak.

## Parent

[[slice-709-check-file-budget]]
