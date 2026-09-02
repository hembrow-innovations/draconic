---
id: "r02-04-default-policy-documented"
title: "R02.04 Default policy documented (permissive vs locked-down as designed)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:57:40Z"
updated_at: "2026-09-02T13:57:40Z"
---

# R02.04 Default policy documented (permissive vs locked-down as designed)

## Done

ROADMAP R02.04 is implemented test-first on both targets: fixtures under `tests/conformance/fixtures/security/permissions` lock the documented default (permissive vs locked-down as designed) when no explicit grant subset is given; R02.04 is `done`.

## Context

Roadmap ID **R02.04** (Default policy documented (permissive vs locked-down as designed)). Runtime-hardening location: whether the designed default is permissive or locked-down, a Program without explicit grants behaves as that default. H04/H06 already land the host surfaces; this sitting is the default-policy contract of the optional Deno-like model (parent R02). Tests under `tests/conformance` fixtures `security/permissions`. Harness `tests/conformance/tests/permissions.rs`. Mark R02.04 `done` only when those tests are green. Not R02 parent remainder, R02.01 grants, R02.02 deny diagnostics, R02.03 CLI/runtime grant flags, H04/H06 surfaces themselves, R01 embed/eval limits, or R04 panic/abort vs catchable exception.

## Verify

`cargo test -p draconic-conformance --test permissions` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R02.04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R02.04), `tests/conformance/fixtures/security/permissions`, `tests/conformance/tests/permissions.rs`, host default-policy surface as needed for both targets

## Links

[[s-r02-04]] [[ticket-109-r02-04-default-policy-documented-permissive-vs]]
