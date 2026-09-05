---
id: "r02-03-cli-runtime-flags-to-grant"
title: "R02.03 CLI/runtime flags to grant subset (opt-in permissions)"
kind: task
status: ready
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:31:42Z"
updated_at: "2026-09-04T13:31:42Z"
---

# R02.03 CLI/runtime flags to grant subset (opt-in permissions)

## Blocked by

None.

## Done

ROADMAP R02.03 is implemented test-first on both targets: CLI/runtime flags grant a subset of permissions; CLI crate tests and `security/permissions` fixtures lock that a granted subset is honoured and an ungranted host op still fails closed; R02.03 is `done`.

## Context

Roadmap ID **R02.03** (`CLI/runtime flags to grant subset (opt-in permissions)`). Runtime-hardening location: opt-in permission grants are invokable from the CLI and runtime so a granted subset of fs/net capabilities is not an embed-only knob. H04/H06 already land the host surfaces; this sitting is the flag surface of the optional Deno-like model (parent R02). Tests under `crates/draconic-cli` (`--test permissions`) and `tests/conformance` fixtures `security/permissions` (harness `tests/conformance/tests/permissions.rs`). Mark R02.03 `done` only when those tests are green. Not R02 parent remainder, R02.01 grants, R02.02 deny diagnostics, R02.04 default policy docs, H04/H06 surfaces themselves, R01 embed/eval limits, or R04 panic/abort vs catchable exception.

## Verify

`cargo test -p draconic-cli --test permissions` prints `test result: ok.` `cargo test -p draconic-conformance --test permissions` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R02.03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R02.03), `crates/draconic-cli`, `tests/conformance/fixtures/security/permissions`, `tests/conformance/tests/permissions.rs`, CLI/runtime grant-flag surface as needed for both targets

## Links

[[s-r02-03]] [[ticket-108-r02-03-cli-runtime-flags-to-grant]]
