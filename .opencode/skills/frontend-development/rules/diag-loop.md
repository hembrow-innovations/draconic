---
title: Frontend diagnosis loop
impact: HIGH
impactDescription: Hypotheses without a red loop waste the session
tags: [diag, debug, loop]
---

## Frontend diagnosis loop

A discipline for hard UI bugs and frontend performance. Skip phases only when explicitly justified. The skill is the **tight red loop**. No loop, no hypothesis.

1. **Build a feedback loop.** One command you have already run. Prefer a failing test at the seam (Vitest, RTL, Playwright, Maestro), then curl, then a harness. Tighten: faster, sharper, deterministic. Flakes: raise reproduction rate until it is debuggable.
2. **Reproduce and minimise.** The loop shows the user's symptom. Cut inputs until every remaining piece is load-bearing.
3. **Hypothesise.** Rank 3 to 5 falsifiable predictions. Show the list. Do not block if the user is AFK.
4. **Instrument.** One variable at a time. Debugger first, then tagged logs such as `[DEBUG-a4f2]`. Perf: measure first.
5. **Fix.** Regression test at a real seam before the fix when that seam exists. Re-run the original loop.
6. **Cleanup.** Remove tagged logs and throwaway harnesses. State the winning hypothesis.

**Incorrect:** reading the component tree to invent a cause, then patching, then asking for a repro.

**Correct:** name the command, paste red output, shrink the repro, then change code.

**Notes.** If you cannot build a loop, stop and say so. Load **tdd**, **vitest**, **react-testing**, **playwright-cli**, or **maestro** for the runner. Do not proceed to Phase 3 without a red-capable command.
