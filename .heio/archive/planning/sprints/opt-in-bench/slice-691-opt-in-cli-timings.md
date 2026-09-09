---
id: "slice-691-opt-in-cli-timings"
title: "Opt-in CLI timings"
kind: slice
status: abandoned
sprint: "opt-in-bench"
blocked_by: []
tags: []
created_at: "2026-09-08T18:55:55Z"
updated_at: "2026-09-09T12:30:00Z"
---
# Opt-in CLI timings

## Why

There is no way to time Draconic compile or program run on demand. Correctness stays on `cargo test --workspace`. This cut adds two representative programs and a hyperfine recipe so you can print compile time and run time on js and native without joining the ten-minute oracle or claiming a latency SLA.

## Done

`tests/perf/compile_heavy.drac` and `tests/perf/run_heavy.drac` build and run on `--target js` and `--target native`. `tests/perf/bench.sh` times `draconic build` with `-o` into a temp dir, then times the artifact, for both programs and both backends. It fails closed with `hyperfine not found` when that host tool is missing. [[performance]] documents the recipe and still is not a production latency SLA. Wall-clock results stay uncommitted. `tests/perf` is not a workspace member.

## Blocked by

None.

## Non-goals

- **ROADMAP row**: Loop stays language completeness
- **CI fail on slowdown**: not a merge gate
- **Criterion / cargo bench crate / `draconic bench`**
- **`cargo test --workspace` as this slice's oracle**
- **Committed timing numbers**
- **GC churn program / conformance-fixture reuse**
- **Rewriting heio locations**

## Oracle checklist

- [ ] O1: both programs build and run on js and native
  CHECK: cargo run -p draconic-cli --quiet -- build --target js tests/perf/compile_heavy.drac -o /tmp/draconic-perf-compile.js && node /tmp/draconic-perf-compile.js && cargo run -p draconic-cli --quiet -- build --target native tests/perf/compile_heavy.drac -o /tmp/draconic-perf-compile && /tmp/draconic-perf-compile && cargo run -p draconic-cli --quiet -- build --target js tests/perf/run_heavy.drac -o /tmp/draconic-perf-run.js && node /tmp/draconic-perf-run.js && cargo run -p draconic-cli --quiet -- build --target native tests/perf/run_heavy.drac -o /tmp/draconic-perf-run && /tmp/draconic-perf-run
  EXPECT: perf-ok
  EVIDENCE: pending

- [ ] O2: script refuses when hyperfine is missing
  CHECK: env PATH=/nonexistent HOME=/nonexistent /bin/bash tests/perf/bench.sh
  EXPECT: hyperfine not found
  EVIDENCE: pending

- [ ] O3: performance note points at the script
  CHECK: rg -n "tests/perf/bench.sh" docs/non-functional/performance.md
  EXPECT: tests/perf/bench.sh
  EVIDENCE: pending

- [ ] O4: tests/perf is not a workspace member
  CHECK: python3 -c "from pathlib import Path; t=Path('Cargo.toml').read_text(); assert 'tests/perf' not in t; print('perf-not-member')"
  EXPECT: perf-not-member
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- [[task-692-perf-fixture-programs]]
- [[task-693-bench-script-and-docs]]

## See also

[[performance]], [[0012-oracle-check-timeout]], [[api-cli]], [[guides-toolchain]], [[ticket-773-native-console-log]], ROADMAP.md (no new row), Cargo.toml workspace members.


## Abandon

ABANDON: [[ticket-773-native-console-log]] — O1 cannot hold. Native LLVM has no console.log lowering, so both-target perf fixtures cannot print. Drain stopped. Tasks stay linked. Do not mark met.
