---
id: "ticket-773-native-console-log"
title: "Native LLVM has no console.log lowering"
kind: ticket
status: open
ticket_type: bug
tags: []
blocked_by: []
created_at: "2026-09-08T19:49:50Z"
updated_at: "2026-09-11T17:00:00Z"
---

# Native LLVM has no console.log lowering

## Signal

[[task-692-perf-fixture-programs]] could not land both-target `tests/perf` programs. `let console = globalThis.console; console.log("perf-ok");` builds and prints on js. Native build of the same print, and of `examples/shebang/hello.drac`, fails with unsupported IR. Host `stdoutWrite` can print on both backends, but that adapter rejects function decls and `for` loops.

## Fit

This project, not [[slice-691-opt-in-cli-timings]]. Slice drain stopped. Task stays `ready` with a Blocked section. Compiler work is out of that fixtures-only unit.

## Notes

- **JS**: console.log works.
- **Native**: console.log is not in the LLVM walk. Re-checked 2026-09-11 after `emit_es_expr_walk`: 100 function decls plus a numeric `let` run; a tight integer `while` runs; neither can share a program with `console.log` or `stdoutWrite`. Native prints observed number (and some string) `let` slots instead of writing stdout.
- **Unblock for slice-691**: native lowering for `globalThis.console.log`, or one walk that allows many functions plus a tight loop plus real stdout.

## Parent

[[slice-691-opt-in-cli-timings]]
