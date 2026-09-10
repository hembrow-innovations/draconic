---
id: "slice-783-llvm-one-emitter"
title: "LLVM one Emitter and SlotTy"
kind: slice
status: active
sprint: "dragons-audit"
blocked_by: []
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T05:20:00Z"
---

# LLVM one Emitter and SlotTy

## Why

Shared `emitter.rs` exists and almost no adapter uses it. Copies of `escape_llvm_string` and `SlotTy` are the prefactor that makes a later one-walker sitting possible. This slice blocks [[slice-784-llvm-no-fingerprint]].

## Done

`escape_llvm_string` lives only in `emitter.rs`. Adapters that only need Number / BigInt / Boolean / String / Undefined use crate `SlotTy` and `Emitter`. llvm tests stay green. No new file over 1000 lines. No new nested classify/ok/eval split.

## Blocked by

None.

## Non-goals

- **deleting try_folded_walks** (that is [[slice-784-llvm-no-fingerprint]])
- **inkwell / llvm-sys**
- **file-budget splits to dodge 1000**

## Oracle checklist

- [ ] O1: one escape helper
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-backend-llvm/src"); hits=[str(p.relative_to(root)) for p in root.rglob("*.rs") if p.name!="emitter.rs" and "fn escape_llvm_string" in p.read_text()]; print("one-escape" if not hits else "dup:"+ ",".join(hits))'
  EXPECT: one-escape
  EVIDENCE: pending
- [ ] O2: llvm backend tests green
  CHECK: cargo test -p draconic-backend-llvm --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- [[task-790-emitter-escape]]
- [[task-791-emitter-slotty-migrate]]

## See also

[[ticket-777-llvm-file-budget-shards]], [[ticket-801-llvm-escape-variants]], [[slice-784-llvm-no-fingerprint]], crates/draconic-backend-llvm/src/emitter.rs
