---
id: "ticket-703-runtime-rust-dev-misses"
title: "Runtime lib.rs abi.rs host_abi_tests.rs over budget"
kind: ticket
status: closed
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# Runtime lib.rs abi.rs host_abi_tests.rs over budget

## Signal

Rust-development review of crates/draconic-runtime. host_abi_tests.rs 5143, lib.rs 3897, abi.rs 3270, all over 1250. crypto, testing, and url are inline pub mod blocks in lib.rs, not sibling files.

## Fit

Promoted to [[slice-714-runtime-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

Host registry, permissive default, abort vs HostError hold. C sources exempt. Split *_tests.rs pattern is allowed.

## Parent

[[slice-714-runtime-file-budget]]
