---
id: "slice-784-llvm-no-fingerprint"
title: "LLVM one IR walker no fingerprints"
kind: slice
status: active
sprint: "dragons-audit"
blocked_by: [ "slice-783-llvm-one-emitter" ]
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T23:59:00Z"
---
# LLVM one IR walker no fingerprints

## Why

Native emit still claims modules by fingerprint. Unsupported IR is a miss in `try_folded_walks`, not a miss in a statement match. Annex B async methods, private methods, and static private fields fail for that reason. ADR-0002 wants one lowerer behind `emit_llvm_ir`.

## Done

`try_folded_walks` is gone. `emit_es_expr_walk` lowers IR with one walker (plus the existing native-ints and empty-hello paths in `emit_llvm_ir_raw`). Host calls go through the catalog, not per-file `walk_host_*`. The three annex-b fixtures from [[ticket-775-annex-b-native-false-green]] run on native again. N08.16.35 / 37 / 38 are `done` only if those tests are green.

## Blocked by

[[slice-783-llvm-one-emitter]]: one Emitter first.

## Non-goals

- **inkwell / llvm-sys**
- **new language behaviour beyond restoring those three native fixtures**
- **splitting adapters to meet 1000 lines**
- **hello-stub success for unsupported IR**

## Oracle checklist

- [x] O1: no try_folded_walks
  CHECK: python3 -c 'from pathlib import Path; t=Path("crates/draconic-backend-llvm/src/es_expr.rs").read_text(); print("no-fold" if "fn try_folded_walks" not in t else "still-fold")'
  EXPECT: no-fold
  EVIDENCE: no-fold (task-793; rechecked task-794)
- [x] O2: annex_b including those three native fixtures green
  CHECK: cargo test -p draconic-conformance --test annex_b --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 104 passed (task-794)
- [x] O3: llvm backend tests green
  CHECK: cargo test -p draconic-backend-llvm --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 321 passed (task-794)

## Pool

- [[task-792-fold-host-catalog-emit]]
- [[task-793-delete-try-folded-walks]]
- [[task-794-annex-b-native-through-walker]]

## See also

[[ticket-776-llvm-fingerprint-adapters]], [[ticket-775-annex-b-native-false-green]], [[ticket-802-llvm-host-emit-bodies]], [[slice-782-annex-b-native-honesty]], ADR-0002, crates/draconic-backend-llvm/src/es_expr.rs
