---
id: "task-816-llvm-walker-native-date-stdlib"
title: "LLVM walker native Date and stdlib"
kind: task
status: ready
mode: afk
blocked_by: ["task-814-host-name-catalog"]
sprint: "platform"
slice: "slice-815-llvm-walker-native-date-stdlib"
tags: []
created_at: "2026-09-12T00:15:00Z"
updated_at: "2026-09-12T00:15:00Z"
---

# LLVM walker native Date and stdlib

## Blocked by

[[task-814-host-name-catalog]]: host dispatch must already be catalog-driven.

## Done

Native Conformance for Date, flags, url, and compression-invalid is green. Unsupported IR still diagnostics.

## Context

Eight native runs fail at `emit_llvm_ir` with unsupported IR while ROADMAP N08.14.06, L04, L07, and L08 are `done`. JS and direct builtin emit stay green. Date.now may steal-route into host time too early. `parseFlags` / `parseUrl` / `gzip` are not walker ident gates. Compression invalid uses `try`. Restore native observations those rows already promised.

## Verify

Slice O1, O2, O3, and O4.

scope: draconic-backend-llvm walker routing and existing Date/stdlib/exception emit; conformance fixtures stay the oracle

## Links

[[slice-815-llvm-walker-native-date-stdlib]] [[ticket-807-llvm-walker-native-date-stdlib]] [[task-814-host-name-catalog]]

## Agent Brief

**Category:** bug
**Summary:** Native Date and stdlib flags/url/compression programs emit instead of unsupported IR.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: ROADMAP N08.14.06, L04, L07, L08 already `done`; restore native observations those rows promised
- Purpose: [[architecture-backend-llvm]] unmatched IR diagnostics, not silent hello
- Contract-first: existing Conformance fixtures are the promise; do not invent new stdlib APIs

**Current behavior:**
`emit_llvm_ir` returns unsupported IR for native `es/builtins/date`, `stdlib/flags`, `stdlib/url`, and `stdlib/compression` invalid. JS target runs. Direct `emit_es_builtins` classification stays green. Date.now can steal-route into host time. Flags/url/gzip idents are missing from walker ES-kind gates. Compression invalid has `try` and may miss exceptions emit.

**Desired behavior:**
Those four native Conformance surfaces pass. The walker selects existing native emit for Date, flags, url, and compression/exceptions. Unmatched IR still returns a Diagnostic. No hello-stub success for non-empty unsupported programs. Do not add host APIs. Do not retarget ROADMAP rows (already `done`). File budget still holds.

**Key interfaces:**
- `emit_llvm_ir` / walker `es_kind` / host steal-route
- existing Date builtin emit, flags/url/compression emit, exceptions emit
- Conformance tests `date_runs`, stdlib_flags, stdlib_url, `invalid_runs_both_targets`
- size-file-budget: no file over 1000

**Acceptance criteria:**
- [ ] Slice O1 `date_runs` prints `test result: ok.`
- [ ] Slice O2 stdlib_flags prints `test result: ok.`
- [ ] Slice O3 stdlib_url prints `test result: ok.`
- [ ] Slice O4 `invalid_runs_both_targets` prints `test result: ok.`
- [ ] Unmatched IR still diagnostics (no hello-stub success)
- [ ] No new host API

**Out of scope:**
- splitting the walker only to meet 1000 lines ([[ticket-809-llvm-walk-over-file-budget]])
- host name catalog ([[ticket-810-host-name-shotgun]])
- inkwell
- marking E17.02 or E18.44 done
- marking N08.14.06 / L04 / L07 / L08 anything other than already `done`
