---
id: "s-l07-02-workspace-timeout"
title: "L07.02 workspace tests finish"
kind: slice
status: failed
sprint: "platform"
tags: []
created_at: "2026-09-05T00:40:51Z"
updated_at: "2026-09-05T01:08:18Z"
claimed-by: 1485841c-6182-422b-ac20-35a6704f9853
---

# L07.02 workspace tests finish

## Why

Review of [[s-l07-02]] left ROADMAP L07.02 unfinished: O1 (`stdlib_flags`) held, but O2 `cargo test --workspace` timed out at 120s. The stdlib location still needs the L07.02 Loop to leave the workspace green, not only the typed-options (bool/string/number) and help-text fixtures.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L07.02 work. The stdlib flags conformance tests stay green. If the hang comes from the L07.02 change, fix that typed-options / help-text surface so the workspace check and those fixtures hold. Mark L07.02 `done` only when those tests are green.

## Blocked by

None. The ticket names no dependency.

## Non-goals

- **Re-opening [[s-l07-02]]**: that slice stays sealed `failed`
- **L07 parent remainder**: umbrella argv → typed options/positionals row
- **L07.01**: parse long/short flags + positionals from a string array
- **H01**: process args/env/exit (host argv, not stdlib parse)
- **L05**: in-language test framework
- **L06**: logging
- Node `util.parseArgs` identity or a full GNU getopt clone

## Oracle checklist

- [ ] O1: workspace tests finish after the L07.02 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=101 match=yes bytes=129492 at=2026-09-05T01:06:57.356Z
  ABANDON: leftover after --reverify; CHECK failed (exit=101 match=yes bytes=129492) → [[ticket-188-l07-02-workspace-tests]]

- [x] O2: L07.02 typed-options and help-text fixtures stay locked by the stdlib flags conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_flags
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=bbb4bc068b85d9aa bytes=3048 at=2026-09-05T01:06:58.111Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l07-02-workspace-timeout]]`

## See also

ROADMAP.md L07.02, `tests/conformance/tests/stdlib_flags.rs`, `tests/conformance/fixtures/stdlib/flags`, CONTEXT.md, [[stdlib]], [[s-l07-02]], [[ticket-185-l07-02-workspace-timeout]], [[ticket-188-l07-02-workspace-tests]].
