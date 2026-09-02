---
id: "d05-02-lto-flag-size-delta"
title: "D05.02 LTO flag documented; size delta smoke"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:15:40Z"
updated_at: "2026-09-02T18:13:10Z"
---

# D05.02 LTO flag documented; size delta smoke

## Done

ROADMAP D05.02 is implemented test-first on the native target: the LTO (or designed) flag is documented and invokable from the CLI, a size-delta smoke compares LTO versus default native artifacts, and D05.02 is `done`.

## Context

Roadmap ID **D05.02** (LTO (or designed) flag documented; size delta smoke). Distribution already produces host binaries; this sitting makes the LTO seam honest at `draconic build` instead of cargo folklore. CLI crate tests under `crates/draconic-cli` lock that the flag is documented and invokable (`lto_flag`). Integration tests under `tests/integration` lock a size-delta smoke of LTO versus default native artifacts (`binary_size_lto`). Mark D05.02 `done` only when those tests are green. Not D05.01 strip-symbols, D05 parent remainder, D03 byte identity, or U07 DWARF mapping.

## Verify

`cargo test -p draconic-cli --test lto_flag` prints `test result: ok.` `cargo test -p draconic-integration-tests --test binary_size_lto` prints `test result: ok.` Workspace `cargo test --workspace` stays green. D05.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (D05.02), `crates/draconic-cli`, `tests/integration`

## Links

[[s-d05-02]] [[ticket-103-d05-02-lto-or-designed-flag-documented]]
