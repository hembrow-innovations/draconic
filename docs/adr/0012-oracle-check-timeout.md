# Oracle CHECK timeout is 10 minutes, not a hang detector

Review of Roadmap slices runs `node .pi/skills/oracle/scripts/oracle-check.mjs --reverify` on the slice file. Each `CHECK:` is `spawnSync` with a timeout. That timeout was **120 seconds**.

## What went wrong

Slice oracles use `CHECK: cargo test --workspace`. On this tree that command is a warm multi-minute run: 15 crates, 60 conformance binaries, 32 integration tests, 18 CLI tests, 6 package tests, **669** `.drac` fixtures.

A live sample of `cargo test --workspace --offline` (2026-09-04):

- **120s**: still running, 58 `test result: ok.` lines, ~86KB output, no failures, in `host_tcp`
- **150s**: still running, 64 ok lines, still progressing

Tickets recorded `unmet exit=timeout match=yes bytes≈70k`. `match=yes` means `EXPECT: test result: ok.` was already in the output. Crates had finished green. The suite had not.

That minting loop produced ~65 `*-workspace-timeout` tickets. Follow-up slices did not unhang anything. They rewrote `CHECK:` to `cargo test --workspace --offline --lib --bins` plus the one package test, which **drops** the conformance/integration surface, then marked the slice `met` with no product-code change.

## Decision

- **Default CHECK timeout is 10 minutes** (`600_000` ms) in `oracle-check.mjs`. Six minutes is about a warm-run duration with no rebuild or Hivemind load; ten minutes is the budget. Raise `ORACLE_CHECK_TIMEOUT_MS` (positive milliseconds) if a future workspace run still dies with `exit=timeout` and `match=yes` while tests are progressing.
- **`exit=timeout` with `match=yes` is a budget miss, not a hang.** Do not file it as a product defect. Do not "fix" it by narrowing `CHECK:` to `--lib --bins`.
- **Keep `cargo test --workspace` as the workspace oracle** when that is the promise. Narrow CHECKs only when the slice's Done bar is actually a smaller surface.

Source of the timeout: `ai/skills/workflow/oracle/scripts/oracle-check.mjs` in agentic-core. Dest copy is `.pi/skills/oracle/scripts/oracle-check.mjs` (do not dest-only patch; reinstall the pack). Override: `ORACLE_CHECK_TIMEOUT_MS`.

## Rejected

- **Leave 120s.** It systematically fails this dest's Review lane.
- **Six minutes as the default.** Warm linear extrapolation from the 150s sample is ~5 minutes. No headroom for LLVM rebuilds or a loaded machine.
- **Treat every workspace timeout as a hang to fix in the Roadmap item.** That is how the ticket factory started.
