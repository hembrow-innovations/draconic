---
id: "s-d03-01-workspace-timeout"
title: "D03.01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T14:12:54Z"
updated_at: "2026-09-04T14:35:36Z"
claimed-by: 62e1caaa-e44f-4951-acf4-ec117d2f6783
---

# D03.01 workspace tests finish

## Why

Review of [[s-d03-01]] left ROADMAP D03.01 unfinished: O1 (`reproducibility_expectations`) held, but O2 `cargo test --workspace` timed out at 120s. The distribution location still needs the D03.01 Loop to leave the workspace green, not only the reproducibility docs suite.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D03.01 work. The `reproducibility_expectations` harness stays green. If the hang comes from the D03.01 change, fix that timestamp-and-path documentation surface so both the workspace check and those integration tests hold. Mark D03.01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-d03-01]]**: that slice stays sealed `failed`
- **D03.02**: Same source + pin → byte-identical or documented-equivalent emit
- **D03 parent remainder**: combining docs + emit identity as one umbrella row
- **D02**: Toolchain version pin in `draconic.toml`
- **D01**: Release binaries + install script
- **D04**: Cross-compile matrix and CI jobs
- **D05**: Strip / LTO size flags

## Oracle checklist

- [x] O1: workspace tests finish after the D03.01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test reproducibility_expectations --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=54eab735c6b95f28 bytes=93490 at=2026-09-04T14:35:23.291Z

- [x] O2: D03.01 docs name timestamp and path reproducibility expectations, locked by the integration suite
  CHECK: cargo test -p draconic-integration-tests --test reproducibility_expectations
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=418f0dcef0b787c3 bytes=2961 at=2026-09-04T14:35:23.333Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[d03-01-workspace-timeout]]`

## See also

ROADMAP.md D03.01, `tests/integration/tests/reproducibility_expectations.rs`, `website/install.md`, docs/, CONTEXT.md, [[distribution]], [[s-d03-01]], [[ticket-128-d03-01-workspace-timeout]].
