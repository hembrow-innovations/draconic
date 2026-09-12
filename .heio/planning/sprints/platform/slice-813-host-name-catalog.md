---
id: "slice-813-host-name-catalog"
title: "LLVM walker host names from catalog"
kind: slice
status: frozen
sprint: "platform"
blocked_by: ["slice-811-llvm-walk-file-budget"]
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T00:15:00Z"
---

# LLVM walker host names from catalog

## Why

Adding a host API still means editing Check `HOST_APIS` and a parallel name-string cascade on the LLVM walker. Classification already lives in `lookup_host_api`. This cut makes walker host dispatch use that catalog instead of a second name table.

## Done

The LLVM walker routes host calls through catalog lookup, not independent host-name string lists in the walker. Check remains owner of language names. `catalog_sync` stays green. `draconic-backend-llvm` tests stay green. Native host Conformance that was already green stays green.

## Blocked by

[[slice-811-llvm-walk-file-budget]]: split the walker before rewriting its host dispatch.

## Non-goals

- **native Date / stdlib emit**: [[ticket-807-llvm-walker-native-date-stdlib]]
- **new host APIs**
- **rewriting JS polyfill bodies**
- **inkwell**
- **hello-stub success for unsupported IR**
- **E17.02 / E18.44**: do not mark done

## Oracle checklist

- [ ] O1: walker has no host_has string-list cascade
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-backend-llvm"); hits=[p.name for p in root.rglob("*.rs") if "host_has(&[" in p.read_text()]; print("catalog-route" if not hits else "shotgun:"+ ",".join(sorted(hits)))'
  EXPECT: catalog-route
  EVIDENCE: pending
- [ ] O2: llvm backend tests green
  CHECK: cargo test -p draconic-backend-llvm --offline
  EXPECT: test result: ok.
  EVIDENCE: pending
- [ ] O3: catalog_sync stays green
  CHECK: cargo test -p draconic-check --offline catalog_sync
  EXPECT: test result: ok.
  EVIDENCE: pending
- [ ] O4: existing native host fixtures stay green
  CHECK: cargo test -p draconic-conformance --test host_fs --test host_process --test host_stdio --test host_path --test host_tcp --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- [[task-814-host-name-catalog]]

## See also

[[ticket-810-host-name-shotgun]], [[task-792-fold-host-catalog-emit]], [[task-765-host-catalog-sync]], [[slice-805-llvm-host-emit-bodies]], ADR-0002
