---
id: "s-l07-workspace-timeout"
title: "L07 workspace tests finish"
kind: slice
status: active
sprint: "platform"
tags: []
created_at: "2026-09-05T00:53:39Z"
updated_at: "2026-09-05T05:46:30Z"
claimed-by: 10d6422d-7f5a-47af-9717-3fc108e6680d
blocked-by: ["s-d04-workspace-tests"]
---

# L07 workspace tests finish

## Why

Review of [[s-l07]] left ROADMAP L07 unfinished: O1 (`stdlib_flags`) held, but O2 `cargo test --workspace` timed out at 120s. The stdlib location still needs the L07 Loop to leave the workspace green, not only the argv → typed options/positionals fixtures.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L07 work. The stdlib flags conformance tests stay green. If the hang comes from the L07 change, fix that flags/CLI parse surface so the workspace check and those fixtures hold. Mark L07 `done` only when those tests are green.

## Blocked by

[[s-d04-workspace-tests]] / [[d04-workspace-tests]]. The named GHA blocker [[s-d04-workspace-disabled-gha]] is archive-only / sealed `failed`. Workspace now finishes (ADR-0012 10m CHECK budget) and fails on D04 disabled GHA (`release-artifact.yml`), not on the L07 flags surface. Do not start this Loop until D04 workspace tests pass.

## Non-goals

- **Re-opening [[s-l07]]**: that slice stays sealed `failed`
- **L07.01**: parse long/short flags + positionals from a string array
- **L07.02**: typed options (bool/string/number); help text as designed
- **H01**: process args/env/exit (host argv, not stdlib parse)
- **L05**: in-language test framework
- **L06**: logging
- Node `util.parseArgs` identity or a full GNU getopt clone

## Oracle checklist

- [ ] O1: workspace tests finish after the L07 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: L07 long/short flags, positionals, and typed-options fixtures stay locked by the stdlib flags conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_flags
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task-pool ids. Never drop them.

- `[[l07-workspace-timeout]]`

## See also

ROADMAP.md L07, `tests/conformance/tests/stdlib_flags.rs`, `tests/conformance/fixtures/stdlib/flags`, CONTEXT.md, [[stdlib]], [[s-l07]], [[ticket-186-l07-workspace-timeout]].
