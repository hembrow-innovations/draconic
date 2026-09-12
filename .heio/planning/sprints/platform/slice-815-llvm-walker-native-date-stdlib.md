---
id: "slice-815-llvm-walker-native-date-stdlib"
title: "LLVM walker native Date and stdlib"
kind: slice
status: met
sprint: "platform"
blocked_by: ["slice-813-host-name-catalog"]
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T22:30:00Z"
---

# LLVM walker native Date and stdlib

## Why

ROADMAP marks native Date and stdlib flags/url/compression `done`, but eight native Conformance runs still fail at `emit_llvm_ir` with unsupported IR. The JS path and direct builtin emit stay green. This cut restores native observations those rows already promised.

## Done

Native Conformance for `es/builtins/date`, `stdlib/flags`, `stdlib/url`, and `stdlib/compression` invalid input is green. Unmatched IR still diagnostics. No hello-stub success. ROADMAP N08.14.06, L04, L07, and L08 stay `done` because the promise already existed.

## Blocked by

[[slice-813-host-name-catalog]]: new walker routing goes through the catalog, not a new name list.

## Non-goals

- **file budget split**: [[ticket-809-llvm-walk-over-file-budget]]
- **host name catalog**: [[ticket-810-host-name-shotgun]]
- **new host APIs**
- **inkwell**
- **hello-stub success for unsupported IR**
- **E17.02 / E18.44**: do not mark done

## Oracle checklist

- [x] O1: native Date fixture runs
  CHECK: cargo test -p draconic-conformance --test builtins --offline date_runs
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-conformance --test builtins --offline date_runs → test result: ok. 1 passed; 0 failed.
- [x] O2: native flags fixtures run
  CHECK: cargo test -p draconic-conformance --test stdlib_flags --offline
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-conformance --test stdlib_flags --offline → test result: ok. 6 passed; 0 failed.
- [x] O3: native url fixtures run
  CHECK: cargo test -p draconic-conformance --test stdlib_url --offline
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-conformance --test stdlib_url --offline → test result: ok. 6 passed; 0 failed.
- [x] O4: native compression invalid fixture runs
  CHECK: cargo test -p draconic-conformance --test stdlib_compression --offline invalid_runs_both_targets
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-conformance --test stdlib_compression --offline invalid_runs_both_targets → test result: ok. 1 passed; 0 failed.

## Pool

- [[task-816-llvm-walker-native-date-stdlib]]

## See also

[[ticket-807-llvm-walker-native-date-stdlib]], ROADMAP N08.14.06 L04 L07 L08, ADR-0002, ADR-0012
