---
id: "r02-01-permission-grants-fs-read-write"
title: "R02.01 Permission grants: fs read/write, net listen/connect (as designed)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:55:00Z"
updated_at: "2026-09-02T13:55:00Z"
---

# R02.01 Permission grants: fs read/write, net listen/connect (as designed)

## Done

ROADMAP R02.01 is implemented test-first on both targets: granted fs read/write and net listen/connect host ops succeed as designed; `security/permissions` fixtures are green and R02.01 is `done`.

## Context

Roadmap ID **R02.01** (Permission grants: fs read/write, net listen/connect (as designed)). Runtime-hardening location: the grant path of the optional Deno-like model (parent R02). H04/H06 already land the host surfaces; this sitting locks that a designed grant makes those ops succeed. Tests under `tests/conformance` fixtures `security/permissions`. Harness `tests/conformance/tests/permissions.rs`. Mark R02.01 `done` only when those tests are green. Not R02 parent remainder, R02.02 deny diagnostics, R02.03 CLI/runtime grant flags, R02.04 default policy docs, H04/H06 surfaces themselves, R01 embed limits, or R04 panic policy.

## Verify

`cargo test -p draconic-conformance --test permissions` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R02.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R02.01), `tests/conformance/fixtures/security/permissions`, `tests/conformance/tests/permissions.rs`, host permission-grant surface as needed for both targets

## Links

[[s-r02-01]] [[ticket-106-r02-01-permission-grants-fs-read-write]]
