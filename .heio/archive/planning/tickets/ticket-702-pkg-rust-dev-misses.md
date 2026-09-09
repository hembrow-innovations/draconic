---
id: "ticket-702-pkg-rust-dev-misses"
title: "Pkg lib.rs cache.rs resolve.rs over file budget"
kind: ticket
status: closed
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# Pkg lib.rs cache.rs resolve.rs over file budget

## Signal

Rust-development review of crates/draconic-pkg. lib.rs 1856 and cache.rs 1277 over 1250. resolve.rs 1121 over 1000.

## Fit

Promoted to [[slice-713-pkg-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

Hand-written Error enums, no serde-for-toml, toml 0.8 hand-walk. Other pkg files in budget.

## Parent

[[slice-713-pkg-file-budget]]
