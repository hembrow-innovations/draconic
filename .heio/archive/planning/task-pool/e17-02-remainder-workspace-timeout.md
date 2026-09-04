---
id: "e17-02-remainder-workspace-timeout"
title: "E17.02 remainder workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T15:07:39Z"
updated_at: "2026-09-04T15:48:42Z"
---

# E17.02 remainder workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP E17.02 remainder work; the `legacy` harness stays green.

## Context

Roadmap ID **E17.02** (Other non-strict legacy beyond `with`; children below; untracked remainder stays here). Review of [[s-e17-02-remainder]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (legacy harness) stayed green. If the hang comes from the E17.02 remainder change, fix that remainder so both the workspace check and the `legacy` harness hold. Leave E17.02 `todo` while untracked remainder remains. Mark E17.02 `done` only when no untracked remainder stays. Not E17.01 `with` statement basics, E18.44 untracked ECMA-262 remainder, N08.15 native observations of non-strict legacy, or Test262 full allowlist. Do not re-open [[s-e17-02-remainder]] or [[s-e17-02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test legacy --offline && cargo test -p draconic-conformance --test harness --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test legacy` still prints `test result: ok.` E17.02 stays `todo` on ROADMAP.md unless no untracked remainder remains.

scope: `ROADMAP.md` (E17.02 / remainder child), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, js frontend/runtime as needed to unhang workspace tests after the remainder

## Links

[[s-e17-02-remainder-workspace-timeout]] [[ticket-137-e17-02-remainder-workspace-timeout]] [[s-e17-02-remainder]]
