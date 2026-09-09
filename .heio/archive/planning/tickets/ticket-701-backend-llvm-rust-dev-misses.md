---
id: "ticket-701-backend-llvm-rust-dev-misses"
title: "LLVM backend many files over budget plus extra globals"
kind: ticket
status: closed
ticket_type: observation
tags: []
sprint: "rust-dev-audit"
blocked_by: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# LLVM backend many files over budget plus extra globals

## Signal

Rust-development review of crates/draconic-backend-llvm. 37 of 76 .rs files over 1000; 26 over 1250. Worst: es_builtins.rs 5194, lib.rs 4937. Extra thread-locals REGEXP_STATICS, FN_REG, SINK_LINES. Process-global OnceLock TOOLS and AtomicU64 id counters.

## Fit

Promoted to [[slice-712-backend-llvm-file-budget]]. Layout and rust-development misses, not a ROADMAP atom.

## Notes

No inkwell/llvm-sys. CURRENT_THIS / CURRENT_NEW_TARGET are allowed. Same-file tests hold.

## Parent

[[slice-712-backend-llvm-file-budget]]
