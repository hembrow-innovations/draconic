---
id: "r02-02-deny-path-clear-diagnostic"
title: "R02.02 Deny path: clear diagnostic when host op lacks grant"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:54:36Z"
updated_at: "2026-09-02T13:54:36Z"
---

# R02.02 Deny path: clear diagnostic when host op lacks grant

## Done

ROADMAP R02.02 is implemented test-first on both targets: fixtures under `tests/conformance/fixtures/security/permissions` lock that a host op without a grant fails with a clear diagnostic; R02.02 is `done`.

## Context

Roadmap ID **R02.02** (Deny path: clear diagnostic when host op lacks grant). Runtime-hardening location: when a host op lacks a grant, the Program gets a clear diagnostic instead of a silent success or an opaque failure. H04/H06 already land the host surfaces; this sitting is the deny diagnostic of the optional Deno-like model (parent R02). Tests under `tests/conformance` fixtures `security/permissions`. Harness `tests/conformance/tests/permissions.rs`. Mark R02.02 `done` only when those tests are green. Not R02 parent remainder, R02.01 grants, R02.03 CLI/runtime grant flags, R02.04 default policy docs, H04/H06 surfaces themselves, R01 embed/eval limits, or R04 panic/abort vs catchable exception.

## Verify

`cargo test -p draconic-conformance --test permissions` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R02.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R02.02), `tests/conformance/fixtures/security/permissions`, `tests/conformance/tests/permissions.rs`, host permission deny path as needed for both targets

## Links

[[s-r02-02]] [[ticket-107-r02-02-deny-path-clear-diagnostic-when]]
