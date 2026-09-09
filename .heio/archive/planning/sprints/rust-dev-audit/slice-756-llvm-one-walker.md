---
id: "slice-756-llvm-one-walker"
title: "LLVM one IR-walking lowerer"
kind: slice
status: met
sprint: "rust-dev-audit"
blocked_by: []
tags: []
created_at: "2026-09-09T17:30:00Z"
updated_at: "2026-09-09T23:00:00Z"
---

# LLVM one IR-walking lowerer

## Why

Dragons audit: the LLVM backend is a 66-arm whole-program cascade, not a lowerer. File-budget splits of those adapters would entrench them. Prefactor [[slice-712-backend-llvm-file-budget]] by collapsing adapters into one walk.

## Done

`emit_llvm_ir_raw` dispatches at most native-ints, one IR walker, and empty hello. Fixture `OBS` printers are gone. `emit_llvm_ir` is unchanged. cargo test -p draconic-backend-llvm is green.

## Blocked by

None. This slice blocks [[slice-712-backend-llvm-file-budget]] file splits until the cascade is gone.

## Non-goals

- **inkwell or llvm-sys**
- **new language behaviour or ROADMAP atoms**
- **splitting adapter files to meet 1000 lines** (that is [[slice-712-backend-llvm-file-budget]] after this)

## Oracle checklist

- [x] O1: emit_llvm_ir_raw is not a 66-arm cascade
  CHECK: python3 -c 'from pathlib import Path; t=Path("crates/draconic-backend-llvm/src/lib.rs").read_text(); a=t.find("fn emit_llvm_ir_raw"); b=t.find("fn is_empty_program"); n=t[a:b].count("_module("); print("walker-dispatch" if 0<=n<=4 else "cascade:%d"%n)'
  EXPECT: walker-dispatch
  EVIDENCE: walker-dispatch
- [x] O2: no fixture OBS printers
  CHECK: python3 -c 'from pathlib import Path; hits=[p.name for p in Path("crates/draconic-backend-llvm/src").glob("*.rs") if "const OBS" in p.read_text()]; print("no-obs" if not hits else "obs:"+",".join(hits))'
  EXPECT: no-obs
  EVIDENCE: no-obs
- [x] O3: llvm backend tests green
  CHECK: cargo test -p draconic-backend-llvm --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 314 passed

## Pool

Durable links to task ids. Never drop them.

- [[task-758-llvm-shared-emitter]]
- [[task-759-llvm-default-walker]]
- [[task-760-llvm-fold-functions]]
- [[task-761-llvm-fold-classes]]
- [[task-762-llvm-delete-fixture-printers]]
- [[task-764-llvm-fold-host-fs]]
- [[task-763-llvm-contract-dispatch]]

## See also

[[slice-712-backend-llvm-file-budget]], [[slice-757-host-catalog]], ADR-0002, crates/draconic-backend-llvm/src/lib.rs, crates/draconic-backend-js/src/emit.rs
