---
id: "task-693-bench-script-and-docs"
title: "Add tests/perf/bench.sh and document it"
kind: task
status: ready
mode: afk
blocked_by: ["task-692-perf-fixture-programs"]
sprint: "opt-in-bench"
slice: "slice-691-opt-in-cli-timings"
tags: []
created_at: "2026-09-08T18:55:55Z"
updated_at: "2026-09-08T18:55:55Z"
---

# Add tests/perf/bench.sh and document it

## Blocked by

[[task-692-perf-fixture-programs]]: the script times those two files.

## Done

`tests/perf/bench.sh` times compile and run of both programs on both backends via hyperfine, using a built `draconic` binary and `-o` into a temp dir. [[performance]] points at the script and still is not a latency SLA.

## Context

[[task-692-perf-fixture-programs]] lands the programs. This sitting makes one command and a durable note.

- Host tool is hyperfine, not a Cargo crate. If it is missing, print `hyperfine not found` to stderr and exit non-zero.
- Do not time `cargo run`. Build `draconic-cli` once if needed, then hyperfine `$DRACONIC` or `target/debug/draconic`.
- For each of `compile_heavy.drac` and `run_heavy.drac`, for js and native: time `draconic build --target … -o <temp>` then time `node` (js) or the native binary. Not `draconic run` (that mixes compile and execute).
- Wall-clock output is stdout only. Do not commit result files. Clean the temp dir.
- Extend [[performance]] in place: how to run the script, that it is opt-in, that it is not an SLA, that it must not join `cargo test --workspace`. Do not add a ROADMAP.md row.
- `tests/perf` stays out of workspace members.

## Verify

Slice O2, O3, and O4 hold. O1 still holds.

scope: `tests/perf/bench.sh`, `docs/non-functional/performance.md` (do not rewrite the programs except a script-path comment if required)

## Links

[[slice-691-opt-in-cli-timings]] [[task-692-perf-fixture-programs]] [[performance]] [[0012-oracle-check-timeout]]

## Agent Brief

**Category:** tooling
**Summary:** Hyperfine wrapper plus performance-note recipe for the two tests/perf programs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none (opt-in timings; no language change)
- Purpose: local compile and run numbers, not a production SLA ([[performance]])
- Contract-first: do not invent CI gates or a `draconic bench` CLI

**Current behavior:**
Programs from [[task-692-perf-fixture-programs]] can be timed by hand. No script. [[performance]] covers test-profile cost and the ten-minute oracle, not this recipe.

**Desired behavior:**
One script as specified in Context. Docs name `tests/perf/bench.sh`. Oracle tokens `hyperfine not found`, `tests/perf/bench.sh`, and `perf-not-member` hold.

**Key interfaces:**
- hyperfine (host)
- `draconic build --target js|native -o` ([[api-cli]])
- `node` for js artifacts
- [[performance]] vault note (load **docs** before editing)

**Acceptance criteria:**
- [ ] `tests/perf/bench.sh` exists and is executable
- [ ] Missing hyperfine prints `hyperfine not found` (slice O2)
- [ ] [[performance]] contains `tests/perf/bench.sh` (slice O3)
- [ ] `Cargo.toml` still has no `tests/perf` (slice O4)
- [ ] Script times build `-o` and artifact execute, both programs, both backends
- [ ] Script does not use `cargo run` inside hyperfine
- [ ] No ROADMAP.md row, no Criterion, no committed numbers

**Out of scope:**
- Changing the two `.drac` programs except if a path must match the script
- CI jobs, Gungraun, instruction counts
- Adding `tests/perf` to workspace members
- `cargo test --workspace` as this task's oracle

**Explain this part:**
Timing `cargo run` would count rustc. Timing `draconic run` would mix compile and execute. The script must time a built CLI and a built artifact so the numbers match the sitting's job.
