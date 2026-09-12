---
id: "slice-811-llvm-walk-file-budget"
title: "LLVM walk file budget"
kind: slice
status: frozen
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T00:15:00Z"
---

# LLVM walk file budget

## Why

The LLVM IR walker file is over the thousand-line target again. Closed [[ticket-777-llvm-file-budget-shards]] met the cap by splitting classify/ok siblings. This cut brings the walker back under budget so later host-catalog and Date/stdlib work does not grow one file.

## Done

Every `draconic-backend-llvm` Rust file is at or under 1000 lines. Walker behavior is unchanged. `draconic-backend-llvm` tests stay green.

## Blocked by

None.

## Non-goals

- **native Date / stdlib emit**: [[ticket-807-llvm-walker-native-date-stdlib]]
- **host name catalog**: [[ticket-810-host-name-shotgun]]
- **inkwell**
- **hello-stub success for unsupported IR**
- **E17.02 / E18.44**: do not mark done

## Oracle checklist

- [ ] O1: llvm backend rust files ≤1000 lines
  CHECK: python3 -c 'from pathlib import Path; root=Path("crates/draconic-backend-llvm"); bad=[]; [bad.append("%s:%d" % (p, len(p.read_text().splitlines()))) for p in sorted(root.rglob("*.rs")) if len(p.read_text().splitlines())>1000]; print("max-loc-ok" if not bad else "over:"+",".join(bad))'
  EXPECT: max-loc-ok
  EVIDENCE: pending
- [ ] O2: llvm backend tests green
  CHECK: cargo test -p draconic-backend-llvm --offline
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

- [[task-812-llvm-walk-file-budget]]

## See also

[[ticket-809-llvm-walk-over-file-budget]], [[ticket-777-llvm-file-budget-shards]], [[slice-712-backend-llvm-file-budget]], size-file-budget
