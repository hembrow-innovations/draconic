---
id: "rust-dev-audit"
title: "rust-development crate audit"
kind: sprint
status: closed
tags: []
created_at: "2026-09-09T16:00:00Z"
updated_at: "2026-09-09T23:59:00Z"
---

# rust-development crate audit

## Grouping

Location: none. Swarm review of every crate in crates/ against rust-development, including the 1000-line target and 1250 hard cap. Not a ROADMAP Loop mill. Not the platform sprint.

## Slices in

- [[slice-756-llvm-one-walker]]: collapse LLVM whole-program adapters into one IR walker. Prefactor for 712. blocked_by: none
- [[slice-757-host-catalog]]: one host name table with sync test. Prefactor for 709 host_api split and 714 abi split. blocked_by: none
- [[slice-705-lexer-file-budget]]: lexer lib.rs split and regexp Diagnostic. blocked_by: none
- [[slice-706-parser-file-budget]]: parser context then lib.rs feature split. blocked_by: none
- [[slice-707-ast-file-budget]]: ast types and printer split. blocked_by: none
- [[slice-708-linker-file-budget]]: linker split and diagnostic codes. blocked_by: none
- [[slice-709-check-file-budget]]: one check walk then file split. blocked_by: none
- [[slice-710-ir-file-budget]]: IR split and dump_module vis. blocked_by: none
- [[slice-711-backend-js-file-budget]]: JS emit es_* split and pointer codes. blocked_by: none
- [[slice-712-backend-llvm-file-budget]]: LLVM file budget and extra globals. blocked_by: [[slice-756-llvm-one-walker]]
- [[slice-713-pkg-file-budget]]: pkg lib/cache/resolve split. blocked_by: none
- [[slice-714-runtime-file-budget]]: runtime lib/abi/tests split and inline mods. abi split waits catalog. blocked_by: none
- [[slice-715-cli-file-budget]]: CLI Frontend policy then main/extract/header/test split. blocked_by: none

## Slices out

- crates that passed the review: draconic-frontend, draconic-diagnostics, draconic-embed, draconic-lsp
- ROADMAP language atoms
- cargo test --workspace as this sprint's oracle
