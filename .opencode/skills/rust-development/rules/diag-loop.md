---
title: Rust diagnosis loop
impact: HIGH
impactDescription: Hypotheses without a red loop waste the session
tags: [diag, debug, loop]
---

## Rust diagnosis loop

A discipline for hard Rust bugs and toolchain performance. Skip phases only when explicitly justified. The skill is the **tight red loop**. No loop, no hypothesis.

1. **Build a feedback loop.** One command you have already run. Prefer `cargo test -p <crate> <filter>`, then a conformance fixture, then `cargo run -p draconic-cli --`. Tighten: faster, sharper, deterministic.
2. **Reproduce and minimise.** The loop shows the symptom. Cut the Program until every remaining line is load-bearing.
3. **Hypothesise.** Rank 3 to 5 falsifiable predictions. Show the list. Do not block if the user is AFK.
4. **Instrument.** One variable at a time. Debugger first, then tagged `eprintln!("[DEBUG-a4f2]")`. Perf: measure first.
5. **Fix.** Regression test at a real seam before the fix when that seam exists. Re-run the original loop.
6. **Cleanup.** Remove tagged logs and throwaway harnesses. State the winning hypothesis.

**Incorrect:** reading `check/src/lib.rs` to invent a cause, then patching, then asking for a repro.

**Correct:** name the cargo command, paste red output, shrink the Program, then change code.

**Notes.** If you cannot build a loop, stop and say so. Load **tdd** or **diagnose** for the runner. Do not proceed to Phase 3 without a red-capable command.
