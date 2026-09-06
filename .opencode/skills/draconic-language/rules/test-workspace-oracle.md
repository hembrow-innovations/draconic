---
title: Workspace test is the oracle
impact: HIGH
impactDescription: Narrowing to --lib --bins drops Conformance
tags: [test, cargo, oracle]
---

## Workspace test is the oracle

The workspace promise is `cargo test --workspace`. Do not shrink it to `--lib --bins` to dodge timeouts.

**Incorrect:** rewriting a CHECK to `--lib --bins` after `exit=timeout match=yes`.

**Correct:** `cargo test --workspace` (fast by default; Test262 is a smoke slice unless `DRACONIC_TEST262_FULL=1`). Timeout with `match=yes` is a budget miss, not a hang.

**Notes.** ADR-0012. Default CHECK budget is 10 minutes. Probe new allowlist paths with `draconic-test262` `probe`. See `test-test262-staged` and `size-target-dir`.
