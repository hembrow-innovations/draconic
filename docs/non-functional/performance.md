---
id: performance
title: Performance
kind: non-functional
description: Test-profile debug settings, target/ size cap, workspace oracle timeout, and GC alloc budgets as tested.
status: active
domain: draconic
area: non-functional
tags: [non-functional, performance]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Performance

## Overview

This note records build-test cost controls and resource budgets that exist in the tree: Cargo test profile, `target/` size, the workspace oracle timeout, and GC / Embed alloc budgets. It is not a production latency SLA. Related: [[0012-oracle-check-timeout]], [[guides-loop]], [[architecture-embed]], [[api-embed]], [[security]].

## Requirements

- **Test profile**: workspace `Cargo.toml` `[profile.test]` sets `debug = "line-tables-only"` and `incremental = false`. Line tables keep panic locations. Incremental is off so `target/` stays small. Fat `debug = 2` test binaries were blowing process startup and the workspace oracle.
- **`target/` under 10GB**: [[AGENTS]] requires the `target` directory stay below 10GB.
- **Oracle CHECK timeout is 10 minutes**: default `600_000` ms. `exit=timeout` with `match=yes` is a budget miss, not a hang. Do not “fix” it by narrowing `CHECK:` away from `cargo test --workspace` ([[0012-oracle-check-timeout]]). Override: `ORACLE_CHECK_TIMEOUT_MS` (positive milliseconds) if a future warm run still dies while tests progress.
- **GC alloc budget (R01.02)**: `gc_alloc_budget_tests` in `crates/draconic-runtime`. Default budget `0` is unlimited. When a budget is set, over-budget `draconic_rt_alloc_object` / string alloc returns NULL (fail closed). After collect, alloc can succeed again. Setting budget back to `0` is unlimited.
- **Embed eval budgets (R01)**: `MAX_EVAL_SOURCE_BYTES` is 1 MiB (reject before parse). `DEFAULT_EVAL_ALLOC_BUDGET_BYTES` is 16 MiB of newly allocated strings. Time budget `Duration::ZERO` is unlimited. Exhaustion is a diagnostic, not a catchable JS exception ([[api-embed]]).

## Approach

Keep test binaries lean (`line-tables-only`, no incremental) so `cargo test --workspace` can finish inside the ten-minute oracle. Do not drop conformance or integration crates from that command to make the clock look green ([[guides-loop]]).

Runtime GC budget is a C ABI cap (`draconic_rt_gc_set_alloc_budget` / `draconic_rt_gc_alloc_budget`). Embed eval charges string concat and `typeof` against its alloc budget in `crates/draconic-embed`.

Native LLVM builds are slower than JS emit. That is expected; it is why the oracle budget is minutes, not two minutes.

## Risks

- **Cold LLVM rebuilds** can still press the ten-minute budget. Raise `ORACLE_CHECK_TIMEOUT_MS` rather than shrinking the suite.
- **`target/` growth**: re-enabling incremental test artifacts or full debug info can breach 10GB and slow page-ins. Do not flip `[profile.test]` without re-checking size and oracle time.
- **Unlimited GC (`budget == 0`)** is the default. Caps are opt-in for tests and Embed callers. Do not claim a production heap SLA that the tests do not pin.
- **Embed 16 MiB default** still fails closed when exceeded. Callers who need more must pass `eval_source_with_alloc_budget` / `eval_source_with_limits`.
