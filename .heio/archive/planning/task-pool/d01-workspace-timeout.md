---
id: "d01-workspace-timeout"
title: "D01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T14:02:44Z"
updated_at: "2026-09-04T14:08:29Z"
---

# D01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP D01 work; the release-binaries, install-script, and install-smoke harnesses stay green.

## Context

Roadmap ID **D01** (Release binaries + install script; one-line install to PATH). Review of [[s-d01]] left O4 unmet: `cargo test --workspace` timed out at 120s while O1–O3 (`release_binaries`, `install_script`, `install_smoke`) stayed green. If the hang comes from the D01 change, fix that release-binaries + install-to-PATH surface so both the workspace check and those integration tests hold. Mark D01 `done` only when those tests are green. Not D01.01–D01.03 as separate atoms, D02 toolchain pin, D03 reproducible-build identity, D04 cross-compile matrix, or D05 strip/LTO. Do not re-open [[s-d01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-integration-tests --test release_binaries --offline && cargo test -p draconic-integration-tests --test install_script --offline && cargo test -p draconic-integration-tests --test install_smoke --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-integration-tests --test release_binaries`, `cargo test -p draconic-integration-tests --test install_script`, and `cargo test -p draconic-integration-tests --test install_smoke` still print `test result: ok.` D01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D01), `tests/integration/tests/release_artifact.rs`, `tests/integration/tests/install_script.rs`, `tests/integration/tests/install_smoke.rs`, `scripts/release-artifact.sh`, `scripts/install.sh`, `website/install.md`, release-binaries + install-to-PATH paths as needed to unhang workspace tests after D01

## Links

[[s-d01-workspace-timeout]] [[ticket-126-d01-workspace-timeout]] [[s-d01]]
