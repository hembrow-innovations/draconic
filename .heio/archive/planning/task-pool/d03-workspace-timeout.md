---
id: "d03-workspace-timeout"
title: "D03 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:24:33Z"
updated_at: "2026-09-04T14:39:55Z"
---

# D03 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D03 work; the `reproducible_builds` harness stays green.

## Context

Roadmap ID **D03** (Reproducible builds: same source + pin → documented-equivalent artifacts). Review of [[s-d03]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`reproducible_builds`) stayed green. If the hang comes from the D03 change, fix that same-source-plus-pin documented-equivalent artifacts surface so both the workspace check and those integration tests hold. Mark D03 `done` only when those tests are green. Not D03.01 timestamp/path docs, D03.02 emit identity, D02 toolchain pin, D01 release binaries + install script, D04 cross-compile matrix, or D05 strip/LTO. Do not re-open [[s-d03]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test reproducible_builds --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test reproducible_builds` still prints `test result: ok.` D03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D03), `tests/integration/tests/reproducible_builds.rs`, same-source-plus-pin documented-equivalent artifacts surface as needed to unhang workspace tests after D03

## Links

[[s-d03-workspace-timeout]] [[ticket-130-d03-workspace-timeout]] [[s-d03]]
