---
id: "slice-805-llvm-host-emit-bodies"
title: "LLVM host emit bodies"
kind: slice
status: active
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-11T23:30:00Z"
updated_at: "2026-09-11T13:30:00Z"
---

# LLVM host emit bodies

## Why

[[task-792-fold-host-catalog-emit]] folded host callee classification into `lookup_host_api` on `host_catalog`. [[slice-784-llvm-no-fingerprint]] is already met: `try_folded_walks` is gone and annex-b native is restored. About twenty-five `walk_host_*` functions still each fingerprint a module and emit a full `@main`. This cut puts leftover host emit on the one IR walker via the catalog, then drops those fingerprint bodies.

## Done

Host calls lower through the catalog on the one IR walker. `walk_host_*` fingerprint adapters are gone or no longer emit a full `@main`. Runtime C ABI calls stay. No new `walk_host_*`. Native host Conformance that was already green stays green. `draconic-backend-llvm` tests stay green.

## Blocked by

None.

## Non-goals

- **native console.log**: [[ticket-773-native-console-log]]
- **inkwell**
- **new host APIs**
- **splitting adapters only to meet 1000 lines**
- **hello-stub success for unsupported IR**
- **annex-b private methods**: already restored on [[slice-784-llvm-no-fingerprint]]; not this slice's Done
- **E17.02 / E18.44**: do not mark done

## Oracle checklist

- [x] O1: walk_host fingerprint adapters gone or no full @main
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-backend-llvm"); hits=[p.name for p in root.rglob("*.rs") if "fn walk_host_" in p.read_text() and "define i32 @main" in p.read_text()]; print("no-host-fingerprint" if not hits else "still:"+ ",".join(sorted(hits)))'
  EXPECT: no-host-fingerprint
  EVIDENCE: no-host-fingerprint (task-806 deleted 26 dead walk_host_* adapters)
- [x] O2: llvm backend tests green
  CHECK: cargo test -p draconic-backend-llvm --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 325 passed
- [x] O3: existing native host fixtures stay green
  CHECK: cargo test -p draconic-conformance --test host_fs --test host_process --test host_stdio --test host_path --test host_tcp --offline
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. host_fs 30, host_path 16, host_process 40, host_stdio 16, host_tcp 10

## Pool

- [[task-806-llvm-host-emit-bodies]]

## See also

[[ticket-802-llvm-host-emit-bodies]], [[task-792-fold-host-catalog-emit]], [[slice-784-llvm-no-fingerprint]], [[ticket-773-native-console-log]], ADR-0002
