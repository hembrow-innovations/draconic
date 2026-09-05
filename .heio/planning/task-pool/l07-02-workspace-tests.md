---
id: "l07-02-workspace-tests"
title: "L07.02 workspace tests pass"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-05T01:12:50Z"
updated_at: "2026-09-05T01:33:16Z"
---

# L07.02 workspace tests pass

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L07.02 work; the stdlib flags conformance tests stay green.

## Context

Roadmap ID **L07.02** (`Typed options (bool/string/number); help text as designed`). Review of [[s-l07-02-workspace-timeout]] left O1 unmet: `cargo test --workspace` failed (exit 101) while O2 (`stdlib_flags`) stayed green. The stdlib location still needs the L07.02 Loop to leave the workspace green, not only the typed-options (bool/string/number) and help-text fixtures. If the failure comes from the L07.02 change, fix that typed-options / help-text surface so the workspace check and those fixtures hold. Mark L07.02 `done` only when those tests are green. Not L07 parent remainder, L07.01 long/short flags + positionals, H01 process argv, L05 test framework, L06 logging, Node `util.parseArgs` identity, or a full GNU getopt clone. Do not re-open [[s-l07-02]] or [[s-l07-02-workspace-timeout]]. Do not take [[s-d04-workspace-disabled-gha]] (disabled GHA workflow readers are a different failure).

## Verify

`cargo test --workspace` prints `test result: ok.` and finishes with exit 0. `cargo test -p draconic-conformance --test stdlib_flags` still prints `test result: ok.` L07.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L07.02), `tests/conformance/tests/stdlib_flags.rs`, `tests/conformance/fixtures/stdlib/flags`, typed-options / help-text surface as needed so workspace tests pass after L07.02

## Links

[[s-l07-02-workspace-tests]] [[ticket-188-l07-02-workspace-tests]] [[s-l07-02-workspace-timeout]] [[s-l07-02]]
