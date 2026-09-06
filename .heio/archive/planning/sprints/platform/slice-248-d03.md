---
id: "slice-248-d03"
title: "D03 Reproducible builds: same source + pin → documented-equivalent artifacts"
kind: slice
status: abandoned
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-02T13:01:26Z"
updated_at: "2026-09-06T18:00:00Z"
---

# D03 Reproducible builds: same source + pin → documented-equivalent artifacts

## Why

ROADMAP D03 locks reproducible builds on the distribution location: the same Program source plus the same toolchain pin yields documented-equivalent artifacts. D03.01 documents timestamp and path expectations; D03.02 is the byte-identical or documented-equivalent emit. This cut is the parent row that those ops form one honest same-source-plus-pin surface.

## Done

ROADMAP D03 is implemented test-first on the compiler target. Building the same source with the same toolchain pin produces artifacts that match the documented equivalence contract (byte-identical where promised, otherwise the documented equivalent). Tests under `tests/integration` (`reproducible_builds`) lock that combined surface. Mark D03 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **D03.01**: Document reproducibility expectations (timestamps, paths)
- **D03.02**: Same source + pin → byte-identical or documented-equivalent emit
- **D01**: Release binaries + install script
- **D02**: Toolchain version pin enforce/warn
- **D04**: Cross-compile matrix and CI jobs
- **D05**: Strip / LTO size flags

## Oracle checklist

- [x] O1: D03 same source + pin yields documented-equivalent artifacts
  CHECK: cargo test -p draconic-integration-tests --test reproducible_builds
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=caabe9b167c63768 bytes=2963 at=2026-09-04T14:19:04.672Z

- [ ] O2: workspace tests stay green after the D03 Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=timeout match=yes bytes=79070 at=2026-09-04T14:21:04.680Z
  ABANDON: leftover after --reverify; CHECK timed out at 120s (exit=timeout match=yes bytes=79070); home [[ticket-130-d03-workspace-timeout]]

## Pool

Durable links to task ids. Never drop them.

- [[task-422-d03-reproducible-builds]]

## See also

ROADMAP.md D03, `tests/integration`, CONTEXT.md, [[location-224-distribution]], [[ticket-95-d03-reproducible-builds-same-source-pin]].
