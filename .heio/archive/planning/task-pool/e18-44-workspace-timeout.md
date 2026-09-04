---
id: "e18-44-workspace-timeout"
title: "E18.44 remainder workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:11:50Z"
updated_at: "2026-09-04T15:53:31Z"
---

# E18.44 remainder workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP E18.44 remainder work; the `annex_b` harness stays green.

## Context

Roadmap ID **E18.44** (Untracked ECMA-262 remainder beyond E01–E18 children; file finer rows as discovered; do not drop). Review of [[s-e18-44]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (annex-b harness) stayed green. If the hang comes from the E18.44 remainder change, fix that remainder so both the workspace check and the `annex_b` harness hold. Leave E18.44 `todo` while untracked remainder remains. Mark E18.44 `done` only when no untracked remainder stays. Not E01–E18.43 tracked children, E17.02 other non-strict legacy remainder, S02 / E19.02 Test262 allowlist expansion, or N08.16 native observations of annex-b fixtures. Do not re-open [[s-e18-44]]. Do not drop E18.44 without filing finer rows.

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test annex_b --offline && cargo test -p draconic-conformance --test harness --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test annex_b` still prints `test result: ok.` E18.44 stays `todo` on ROADMAP.md unless no untracked remainder remains.

scope: `ROADMAP.md` (E18.44 / remainder child), `tests/conformance/fixtures/es/annex-b`, `tests/conformance/tests/annex_b.rs`, js frontend/runtime as needed to unhang workspace tests after the remainder

## Links

[[s-e18-44-workspace-timeout]] [[ticket-138-e18-44-workspace-timeout]] [[s-e18-44]]
