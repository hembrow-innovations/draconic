---
id: "ticket-773-native-console-log"
title: "Native LLVM has no console.log lowering"
kind: ticket
status: open
ticket_type: bug
tags: []
blocked_by: []
created_at: "2026-09-08T19:49:50Z"
updated_at: "2026-09-11T18:00:00Z"
---

# Native LLVM has no console.log lowering

## Signal

[[task-692-perf-fixture-programs]] could not land both-target `tests/perf` programs. `let console = globalThis.console; console.log("perf-ok");` builds and prints on js. Native build of the same print, and of `examples/shebang/hello.drac`, fails with unsupported IR. Host `stdoutWrite` can print on both backends, but that adapter rejects function decls and `for` loops.

## Fit

This project, not [[slice-691-opt-in-cli-timings]]. Slice drain stopped. Task stays `ready` with a Blocked section. Compiler work is out of that fixtures-only unit.

## Notes

- **JS**: console.log works.
- **Native**: console.log is not in the LLVM walk. Re-checked after claiming [[task-692-perf-fixture-programs]]: `examples/shebang/hello.drac` and `console.log("perf-ok")` still fail native; `stdoutWrite("perf-ok\n")` still prints on both alone; one function plus `stdoutWrite`, and a `while` plus `stdoutWrite`, still fail native.
- **Unblock for slice-691**: native lowering for `globalThis.console.log`, or one walk that allows many functions plus a tight loop plus real stdout.

## Parent

[[slice-691-opt-in-cli-timings]]
