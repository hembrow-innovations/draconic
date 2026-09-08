---
id: "task-692-perf-fixture-programs"
title: "Add tests/perf compile-heavy and run-heavy programs"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "opt-in-bench"
slice: "slice-691-opt-in-cli-timings"
tags: []
created_at: "2026-09-08T18:55:55Z"
updated_at: "2026-09-08T18:55:55Z"
---

# Add tests/perf compile-heavy and run-heavy programs

## Blocked by

None.

## Done

`tests/perf/compile_heavy.drac` and `tests/perf/run_heavy.drac` each print `perf-ok` after a successful js and native build-and-run.

## Context

No bench fixtures exist. Conformance programs are too small to time. These two files are the workload, not a crate.

- **compile_heavy.drac**: on the order of 100 small functions, trivial main that calls a few, so `draconic build` has real frontend and emit work. Run may be cheap.
- **run_heavy.drac**: tight integer arithmetic loop, little allocation, tens to hundreds of milliseconds of useful work, then print. Compile may be cheap.
- Both targets: `--target js` and `--target native` must succeed. Use only features that already run on both. No host fs/net. Print with `let console = globalThis.console;` as in `examples/shebang/hello.drac`.
- Always `draconic build ... -o` into a temp path. Do not emit `{stem}.out.js` / `{stem}.out` beside the fixtures.
- Do not add a Cargo member. Do not write `bench.sh` or edit [[performance]] here.

## Verify

Slice O1 holds for these two files. Each run prints `perf-ok`. Exit 0 on js (`node` on the JS artifact) and native (the binary).

scope: `tests/perf/compile_heavy.drac`, `tests/perf/run_heavy.drac` only (directory create ok)

## Links

[[slice-691-opt-in-cli-timings]]

## Agent Brief

**Category:** fixtures
**Summary:** Commit two both-target Draconic programs under `tests/perf/` for later hyperfine timings.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none (opt-in timing fixtures; no language change)
- Purpose: local compile and run timings, not a latency SLA ([[performance]])
- Contract-first: do not invent CI gates, Criterion, or a `draconic bench` command

**Current behavior:**
No `tests/perf/` programs. Workspace oracle is correctness only.

**Desired behavior:**
Two committed `.drac` files as specified in Context. Both print `perf-ok`. Both build and run on js and native.

**Key interfaces:**
- `draconic build --target js|native <file> -o <out>` ([[api-cli]])
- JS execute: `node <out>`
- Native execute: the binary at `-o`
- Load **draconic-language** if you need dual-backend constraints

**Acceptance criteria:**
- [ ] `tests/perf/compile_heavy.drac` exists (many small functions, trivial main)
- [ ] `tests/perf/run_heavy.drac` exists (tight numeric loop, little allocation)
- [ ] Slice O1 CHECK prints `perf-ok` and exits 0
- [ ] No workspace member, no script, no docs, no ROADMAP.md edit
- [ ] No scratch `{stem}.out.js` / `{stem}.out` left beside the fixtures

**Out of scope:**
- `tests/perf/bench.sh`
- [[performance]] edits
- Criterion, cargo bench, `draconic bench`
- GC churn; conformance reuse; CI
- `cargo test --workspace` as this task's oracle

**Explain this part:**
Hyperfine will time build versus execute separately. These files must actually run so a slow or failing program is not mistaken for a compiler number.
