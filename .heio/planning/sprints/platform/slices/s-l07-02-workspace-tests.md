---
id: "s-l07-02-workspace-tests"
title: "L07.02 workspace tests pass"
kind: slice
status: released
sprint: "platform"
tags: []
created_at: "2026-09-05T01:11:00Z"
updated_at: "2026-09-05T05:46:30Z"
---

# L07.02 workspace tests pass

## Why

Review of [[s-l07-02-workspace-timeout]] left ROADMAP L07.02 unfinished: O2 (`stdlib_flags`) held, but O1 `cargo test --workspace` failed (exit 101). The stdlib location still needs the L07.02 Loop to leave the workspace green, not only the typed-options (bool/string/number) and help-text fixtures.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L07.02 work. The stdlib flags conformance tests stay green. If the failure comes from the L07.02 change, fix that typed-options / help-text surface so the workspace check and those fixtures hold. Mark L07.02 `done` only when those tests are green.

## Blocked by

None. The ticket names no dependency.

## Non-goals

- **Re-opening [[s-l07-02]] / [[s-l07-02-workspace-timeout]]**: those slices stay sealed `failed`
- **[[s-d04-workspace-disabled-gha]]**: disabled GHA workflow readers are a different failure
- **L07 parent remainder**: umbrella argv → typed options/positionals row
- **L07.01**: parse long/short flags + positionals from a string array
- **H01**: process args/env/exit (host argv, not stdlib parse)
- **L05**: in-language test framework
- **L06**: logging
- Node `util.parseArgs` identity or a full GNU getopt clone

## Oracle checklist

- [ ] O1: workspace tests pass after the L07.02 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: L07.02 typed-options and help-text fixtures stay locked by the stdlib flags conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_flags
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l07-02-workspace-tests]]`

## See also

ROADMAP.md L07.02, `tests/conformance/tests/stdlib_flags.rs`, `tests/conformance/fixtures/stdlib/flags`, CONTEXT.md, [[stdlib]], [[s-l07-02]], [[s-l07-02-workspace-timeout]], [[ticket-188-l07-02-workspace-tests]].
