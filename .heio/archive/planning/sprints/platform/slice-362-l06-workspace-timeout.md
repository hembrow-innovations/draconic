---
id: "slice-362-l06-workspace-timeout"
title: "L06 workspace tests finish"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-04T21:01:32Z"
updated_at: "2026-09-06T18:00:00Z"
---

# L06 workspace tests finish

## Why

Review of [[slice-363-l06]] left ROADMAP L06 unfinished: O1 (`stdlib_logging`) held, but O2 `cargo test --workspace` timed out at 120s. The stdlib location still needs the L06 Loop to leave the workspace green, not only the leveled-logger and stderr/stdout sink fixtures.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L06 work. The stdlib logging conformance tests stay green. If the hang comes from the L06 change, fix that leveled-logger / stdio-sink surface so the workspace check and those fixtures hold. Mark L06 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[slice-363-l06]]**: that slice stays sealed `failed`
- **L06.01**: Leveled log (error/warn/info/debug); filter by level (already `done`)
- **L06.02**: Sink to stderr/stdout (string format) (already `done`)
- **H02**: Host stdio surface
- **L05**: In-language test framework
- structured JSON logs, file sinks, syslog, or a Node `console` identity surface

## Oracle checklist

- [x] O1: workspace tests finish after the L06 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=178d95d073ddab23 bytes=186889 at=2026-09-04T23:21:24.750Z

- [x] O2: L06 leveled-logger and stdio-sink fixtures stay locked by the stdlib logging conformance tests
  CHECK: cargo test -p draconic-conformance --test stdlib_logging
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=305850a20dda20ac bytes=3040 at=2026-09-04T23:21:25.710Z

## Pool

Durable links to task ids. Never drop them.

- `[[task-538-l06-workspace-timeout]]`

## See also

ROADMAP.md L06, `tests/conformance/tests/stdlib_logging.rs`, `tests/conformance/fixtures/stdlib/logging`, `crates/draconic-backend-llvm/src/es_logging.rs`, `crates/draconic-runtime/src/logging.rs`, CONTEXT.md, [[location-223-stdlib]], [[slice-363-l06]], [[ticket-184-l06-workspace-timeout]].
