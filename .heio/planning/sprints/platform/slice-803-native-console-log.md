---
id: "slice-803-native-console-log"
title: "Native console.log lowering"
kind: slice
status: active
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-11T23:30:00Z"
updated_at: "2026-09-11T18:10:00Z"
---

# Native console.log lowering

## Why

[[ticket-773-native-console-log]]: native LLVM has no `globalThis.console.log` lowering. The same print already works on js. Native of that print, and of the documented shebang hello Program, fails with unsupported IR. Host `stdoutWrite` can print on both backends alone, then rejects a function decl or a tight loop in the same Program. That blocked [[task-692-perf-fixture-programs]] (fixtures-only; compiler work is out of that unit). This cut makes native `console.log` print, including next to a function (compile_heavy shape) and next to a tight loop (run_heavy shape).

## Done

A Program that binds `let console = globalThis.console` and calls `console.log` builds and prints on `--target native`. The same print still works when the Program also has a function declaration, and when it also has a tight integer loop. Unsupported IR is not the outcome. A `stdoutWrite`-only workaround is not Done.

## Blocked by

None.

## Non-goals

- **fixtures under `tests/perf`**: that remains [[task-692-perf-fixture-programs]]
- **`bench.sh` / hyperfine recipe**: that remains [[task-693-bench-script-and-docs]]
- **`walk_host_*` fold**: that remains [[ticket-802-llvm-host-emit-bodies]]
- **marking E17.02 or E18.44 done**
- **inkwell / llvm-sys**
- **new host APIs** (`stdoutWrite`, sockets, fs). This is the JS builtin `globalThis.console.log`, not [[location-219-host-io]]
- **`stdoutWrite`-only workaround as Done**
- **`cargo test --workspace` as this slice's oracle** (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)
- **reviving abandoned [[slice-691-opt-in-cli-timings]]**

## Oracle checklist

- [x] O1: native `globalThis.console.log` prints
  CHECK: d=$(mktemp -d) && printf '%s\n' 'let console = globalThis.console;' 'console.log("console-ok");' > "$d/p.drac" && cargo run -p draconic-cli --quiet -- build --target native "$d/p.drac" -o "$d/p" && "$d/p"
  EXPECT: console-ok
  EVIDENCE: exit 0 stdout `console-ok` (task-804)

- [x] O2: a function plus `console.log` still prints on native (unblocks compile_heavy shape)
  CHECK: d=$(mktemp -d) && printf '%s\n' 'function f() { return 1; }' 'let x = f();' 'let console = globalThis.console;' 'console.log("fn-ok");' > "$d/p.drac" && cargo run -p draconic-cli --quiet -- build --target native "$d/p.drac" -o "$d/p" && "$d/p"
  EXPECT: fn-ok
  EVIDENCE: exit 0 stdout contains `fn-ok` (task-804)

- [x] O3: a tight loop plus `console.log` still prints on native (unblocks run_heavy shape)
  CHECK: d=$(mktemp -d) && printf '%s\n' 'let i = 0;' 'while (i < 100) { i = i + 1; }' 'let console = globalThis.console;' 'console.log("loop-ok");' > "$d/p.drac" && cargo run -p draconic-cli --quiet -- build --target native "$d/p.drac" -o "$d/p" && "$d/p"
  EXPECT: loop-ok
  EVIDENCE: exit 0 stdout contains `loop-ok` (task-804)

## Pool

Durable links to task ids. Never drop them.

- [[task-804-native-console-log]]

## See also

[[ticket-773-native-console-log]] [[task-692-perf-fixture-programs]] [[ticket-802-llvm-host-emit-bodies]] [[location-219-host-io]] [[0002-shared-ir-dual-backends]] [[0012-oracle-check-timeout]] [[slice-216-workspace-test-budget]] [[slice-691-opt-in-cli-timings]] [[Toolchain purpose]] [[Host I/O purpose]]
