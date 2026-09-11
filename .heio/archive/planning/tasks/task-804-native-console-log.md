---
id: "task-804-native-console-log"
title: "Lower native console.log through LLVM"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-803-native-console-log"
tags: []
created_at: "2026-09-11T23:30:00Z"
updated_at: "2026-09-11T18:10:00Z"
---

# Lower native console.log through LLVM

## Blocked by

None.

## Done

Native `--target native` of `let console = globalThis.console; console.log(…)` prints the argument. The same print still works in a Program that also has a function declaration, and in a Program that also has a tight integer loop. Slice O1–O3 hold.

## Context

Current: js already builds and prints `let console = globalThis.console; console.log("perf-ok")`. Native of that print, and of the documented shebang hello Program, fails with unsupported IR (no LLVM lowering; empty hello is not used). Host `stdoutWrite` prints on both backends when it is the whole Program, then fails native when a function decl or a tight loop shares the Program. Dual backends share one IR ([[0002-shared-ir-dual-backends]]). `host-io.stdio:read-write` is `stdoutWrite`, not this JS builtin. [[location-219-host-io]] is process/stdio/sockets.

Desired: LLVM lowers `globalThis.console.log` so a portable print Program is not JS-only. Native print must compose with a function (compile_heavy shape) and with a tight loop (run_heavy shape). Do not treat `stdoutWrite` alone as Done.

Out of scope: `tests/perf` fixtures ([[task-692-perf-fixture-programs]]), `bench.sh` ([[task-693-bench-script-and-docs]]), leftover `walk_host_*` emit bodies ([[ticket-802-llvm-host-emit-bodies]]), inkwell, new host APIs, marking E17.02 / E18.44 done.

CHECK timeout for O1–O3 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes.

Always `draconic build … -o` into a temp path. Do not emit `{stem}.out.js` / `{stem}.out` beside fixtures or examples.

## Verify

Slice O1–O3 CHECK print their EXPECT tokens and exit 0. `cargo test -p draconic-backend-llvm` prints `test result: ok.` That crate test, not workspace, is the unit oracle.

scope: LLVM backend (console.log / `globalThis.console` native lowering and crate tests only). Not `tests/perf`, not bench script, not host `walk_host_*` fold, not ROADMAP.md, not [[platform]] shape.md.

## Links

[[slice-803-native-console-log]] [[ticket-773-native-console-log]]

## Agent Brief

**Category:** bug
**Summary:** Lower `globalThis.console.log` on the native LLVM backend so a portable print Program prints, including next to a function and next to a tight loop.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Skills:** load **tdd**, **draconic-language**, **rust-development**. Compiler sitting, not a Roadmap atom and not fixtures-only.

**Intent (required when product behaviour changes):**
- This **is** language/backend behaviour (native `console.log`). **No product behaviour change** is false.
- Promise ids: `toolchain.cli:build-targets` (native writes a runnable binary for a Program js already runs). There is no dedicated `console.log` promise. Do not invent host-io ids. `host-io.stdio:read-write` is `stdoutWrite`, not `globalThis.console.log`. `language.dual-worlds:js-only-hard-error-on-llvm` is for JS-only Programs; documented hello using `globalThis.console.log` is meant to be portable, not a silent JS-only hard error. If a new promise is required, assert it under toolchain/language, not host-io.
- Purpose: [[Toolchain purpose]] and dual backends ([[0002-shared-ir-dual-backends]]). Not [[Host I/O purpose]] / [[location-219-host-io]] (process, stdio, sockets).
- Contract-first: assert native print of `globalThis.console.log` (crate test plus temp native build) → then lower. Do not invent a new host API.

**Current behavior:**
js prints `let console = globalThis.console; console.log(…)`. Native of the same Program fails with unsupported IR. `stdoutWrite` prints on both backends only as the whole Program; a function decl or a tight loop in the same Program still fails native. The documented shebang hello Program fails native for the same reason.

**Desired behavior:**
`--target native` of `let console = globalThis.console; console.log("console-ok")` prints `console-ok` and exits 0. Adding a function declaration still prints. Adding a tight integer `while` (or `for`) still prints. js print stays working. Non-empty unsupported IR still hard-errors; empty-program hello stub is not a success for these Programs.

**Key interfaces:**
- Shared IR `Module` consumed by both backends. Public native lowerer `emit_llvm_ir` (and the existing ES expression walker). Unclassified non-empty IR stays a hard diagnostic, not B08 hello.
- JS builtin `globalThis.console` / `.log`, bound as `let console = globalThis.console`. Free identifier `console` is unresolved unless bound. This is not host `stdoutWrite` / `stderrWrite`.
- CLI: `draconic build --target native <file> -o <temp>`; execute the binary at `-o`. JS execute remains `node` on the JS artifact when checking no-regress.
- Host catalog / Runtime ABI may already print strings for `stdoutWrite`. Reuse real stdout; do not add a new host API name for console.

**Acceptance criteria:**
- [x] Slice O1 CHECK prints `console-ok` and exits 0
- [x] Slice O2 CHECK prints `fn-ok` and exits 0
- [x] Slice O3 CHECK prints `loop-ok` and exits 0
- [x] `cargo test -p draconic-backend-llvm` prints `test result: ok.`
- [x] Native build always uses `-o` into temp; no `{stem}.out.js` / `{stem}.out` beside fixtures or examples
- [x] js of the same `globalThis.console.log` print still works
- [x] Promise ids listed above still hold (or a new toolchain/language promise was asserted, not a host-io id)
- [x] Slice O1–O3 EVIDENCE is filled

## Gauntlet

- **round 1**: slice O1–O3 CHECK plus `cargo test -p draconic-backend-llvm` — win. O1 stdout `console-ok` exit 0. O2 stdout contains `fn-ok` exit 0. O3 stdout contains `loop-ok` exit 0. Crate `test result: ok.` 324 passed. js of the same print still prints `console-ok`. Promise `toolchain.cli:build-targets` holds; no host-io id invented.

**Out of scope:**
- `tests/perf` fixtures ([[task-692-perf-fixture-programs]])
- `bench.sh` / [[performance]] / hyperfine ([[task-693-bench-script-and-docs]])
- Folding leftover `walk_host_*` emit bodies ([[ticket-802-llvm-host-emit-bodies]])
- Marking E17.02 or E18.44 done
- inkwell / llvm-sys
- New host APIs, sockets, fs, grants
- `stdoutWrite`-only as Done
- `cargo test --workspace` as this task’s oracle
- Editing [[platform]] shape.md or ROADMAP.md
- Respect [[Toolchain purpose]] and [[Host I/O purpose]] Out of scope fences
