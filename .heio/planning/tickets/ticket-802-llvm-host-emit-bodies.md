---
id: "ticket-802-llvm-host-emit-bodies"
title: "LLVM host emit bodies still per-domain walk_host"
kind: ticket
status: open
ticket_type: observation
tags: []
created_at: "2026-09-11T22:30:00Z"
updated_at: "2026-09-11T22:30:00Z"
---

# LLVM host emit bodies still per-domain walk_host

## Signal

[[task-792-fold-host-catalog-emit]] folded host callee classification into one catalog helper (`lookup_host_api`). About 25 `walk_host_*` functions still each fingerprint a module and emit a full `@main`. Slice [[slice-784-llvm-no-fingerprint]] still needs those emit bodies on the one IR walker, without new fingerprint adapters.

## Fit

this slice → leftover after task-792 classification-only stop

## Notes

- Classification lives in `crates/draconic-backend-llvm/src/host_catalog.rs`
- `try_folded_walks` host arms were not grown and were not deleted (that is [[task-793-delete-try-folded-walks]])
- Do not add a new host fingerprint adapter
- Keep Runtime C ABI calls
