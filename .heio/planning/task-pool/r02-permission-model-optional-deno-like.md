---
id: "r02-permission-model-optional-deno-like"
title: "R02 Permission model (optional Deno-like): grant/deny fs and net; clear deny diagnostics"
kind: task
status: ready
blocked-by: ["r02-01-permission-grants-fs-read-write", "r02-02-deny-path-clear-diagnostic", "r02-03-cli-runtime-flags-to-grant", "r02-04-default-policy-documented"]
tags: []
created_at: "2026-09-02T22:32:35Z"
updated_at: "2026-09-05T05:46:30Z"
---

# R02 Permission model (optional Deno-like): grant/deny fs and net; clear deny diagnostics

## Blocked by

[[r02-01-permission-grants-fs-read-write]], [[r02-02-deny-path-clear-diagnostic]], [[r02-03-cli-runtime-flags-to-grant]], and [[r02-04-default-policy-documented]]. Parent remainder waits until those child atoms land so Build does not duplicate those Loops.

## Done

ROADMAP R02 is implemented test-first on both targets: fixtures under `tests/conformance/fixtures/security/permissions` lock grant/deny for fs and net, a denied host op produces a clear diagnostic, and R02 is `done`.

## Context

Roadmap ID **R02** (Permission model (optional Deno-like): grant/deny fs and net; clear deny diagnostics). Runtime-hardening location: the parent row that host fs/net ops can be granted or denied as a designed policy. H04/H06 children already land the host ops; this sitting unifies grant/deny plus clear deny diagnostics on both targets. Tests under `tests/conformance` fixtures `security/permissions`. Harness `tests/conformance/tests/permissions.rs`. Mark R02 `done` only when those tests are green. Not R02.01 grants, R02.02 deny diagnostics, R02.03 CLI/runtime grant flags, or R02.04 default policy docs as separate atoms, and not R01 embed limits, R03 supply-chain policy, R04 panic policy, or H04/H06 surfaces themselves.

## Verify

`cargo test -p draconic-conformance --test permissions` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R02), `tests/conformance/fixtures/security/permissions`, `tests/conformance/tests/permissions.rs`, host permission grant/deny surface as needed for both targets

## Links

[[s-r02]] [[ticket-105-r02-permission-model-optional-deno-like]]
