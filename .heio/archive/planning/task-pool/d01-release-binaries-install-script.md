---
id: "d01-release-binaries-install-script"
title: "D01 release binaries + install-to-PATH surface"
kind: task
status: completed
tags: []
created_at: "2026-09-03T02:46:00Z"
updated_at: "2026-09-03T02:50:00Z"
---

# D01 release binaries + install-to-PATH surface

## Done

ROADMAP D01 is implemented test-first on the compiler: CI/release stages a host-triple binary, the install script places `draconic` on PATH, and a fresh PATH can run `draconic -V` and parse a hello Program; `release_binaries` integration tests are green and D01 is `done`.

## Context

Roadmap ID **D01** (Release binaries + install script; one-line install to PATH). D01.01–D01.03 already land the host-triple artifact, install script, and fresh-PATH smoke; this sitting unifies them as one honest release-binaries + install-to-PATH surface. Tests under `tests/integration` (`release_binaries`). Mark D01 `done` only when those tests are green. Not D01.01, D01.02, D01.03, D02, D03, D04, or D05.

## Verify

`cargo test -p draconic-integration-tests --test release_binaries` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D01), `tests/integration/tests/release_binaries.rs`, `scripts/release-artifact.sh`, `scripts/install.sh` as needed for the parent surface

## Links

[[s-d01]] [[ticket-93-d01-release-binaries-install-script-one]]
