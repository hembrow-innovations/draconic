---
id: "d03-02-workspace-tests"
title: "D03.02 workspace tests pass"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:21:47Z"
updated_at: "2026-09-04T14:35:16Z"
---

# D03.02 workspace tests pass

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D03.02 work; the `reproducible_emit` harness stays green.

## Context

Roadmap ID **D03.02** (Same source + pin → byte-identical or documented-equivalent emit). Review of [[s-d03-02]] left O2 unmet: `cargo test --workspace` failed (exit 101) while O1 (`reproducible_emit`) stayed green. Workspace did not compile `draconic-pkg` lib tests: `LaterPackaging` undeclared in `crates/draconic-pkg/src/later.rs`. If the compile failure comes from the D03.02 change, fix that same-source-plus-pin emit surface so both the workspace check and those integration tests hold. If it is `draconic-pkg` `later.rs` (`LaterPackaging` undeclared under lib tests), make that crate compile under workspace tests so both hold. Mark D03.02 `done` only when those tests are green. Not D03.01 timestamp/path docs, D03 parent remainder, D02 toolchain pin, D01 release binaries + install script, D04 cross-compile matrix, or D05 strip/LTO. Do not re-open [[s-d03-02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test reproducible_emit --offline` prints `test result: ok.` `cargo test -p draconic-integration-tests --test reproducible_emit` still prints `test result: ok.` D03.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D03.02), `tests/integration/tests/reproducible_emit.rs`, `crates/draconic-pkg/src/later.rs`, same-source-plus-pin emit surface as needed so workspace tests compile and stay green after D03.02

## Links

[[s-d03-02-workspace-tests]] [[ticket-129-d03-02-workspace-tests]] [[s-d03-02]]
