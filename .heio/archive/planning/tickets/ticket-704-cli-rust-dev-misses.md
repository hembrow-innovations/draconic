---
id: "ticket-704-cli-rust-dev-misses"
title: "CLI main.rs extract tests and helpers over file budget"
kind: ticket
status: closed
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# CLI main.rs extract tests and helpers over file budget

## Signal

Rust-development review of crates/draconic-cli. main.rs 1768 and tests/extract.rs 2343 over 1250. extract.rs 1141, c_header.rs 1021, tests/build.rs 1227 over 1000.

## Fit

Promoted to [[slice-715-cli-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

No clap. Frontend compile_path/check_path. unsafe isatty only. Hand argv match.

## Parent

[[slice-715-cli-file-budget]]
