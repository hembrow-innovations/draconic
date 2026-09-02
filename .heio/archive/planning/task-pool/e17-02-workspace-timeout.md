---
id: "e17-02-workspace-timeout"
title: "E17.02 remainder workspace tests finish"
kind: task
status: completed
tags: []
created_at: "2026-09-02T09:19:43Z"
updated_at: "2026-09-02T10:00:00Z"
---

# E17.02 remainder workspace tests finish

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP E17.02 remainder work; the `legacy` harness stays green.

## Context

Roadmap ID **E17.02** (other non-strict legacy beyond `with`). Review of [[s-e17-02]] left O2 unmet: `cargo test --workspace` timed out at 120s while the legacy harness stayed green. If the hang comes from the E17.02 remainder change, fix that remainder so both checks hold. Leave E17.02 `todo` while untracked remainder remains. Mark E17.02 `done` only when no untracked remainder stays. Not E17.01, E18.44, N08.15, or Test262 full allowlist. Do not re-open [[s-e17-02]].

## Verify

`cargo test --workspace` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test legacy` still prints `test result: ok.` E17.02 stays `todo` on ROADMAP.md unless no untracked remainder remains.

scope: `ROADMAP.md` (E17.02 / remainder child), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, js frontend/runtime as needed to unhang workspace tests after the remainder

## Links

[[s-e17-02-workspace-timeout]] [[ticket-28-e17-02-workspace-timeout]] [[s-e17-02]] [[e17-02-remainder]]
