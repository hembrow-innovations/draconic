---
id: "e18-44-untracked-ecma-262-remainder"
title: "E18.44 one atomic untracked ECMA-262 remainder"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:17:05Z"
updated_at: "2026-09-02T19:50:58Z"
---

# E18.44 one atomic untracked ECMA-262 remainder

## Done

One atomic untracked remainder of ROADMAP E18.44 is implemented test-first on the js target; `tests/conformance` fixtures (typically `es/annex-b`) plus the `annex_b` harness are green for that remainder.

## Context

Roadmap ID **E18.44** (Untracked ECMA-262 remainder beyond E01–E18 children; file finer rows as discovered; do not drop). E18.01–E18.43 stay `done`. This sitting implements one untracked remainder (Annex B / late ES beyond tracked children). If E18.44 is larger than one sitting, split one child under E18.44 and complete only that child; leave E18.44 `todo` while untracked remainder remains. Mark E18.44 `done` only when no untracked remainder stays. Fixtures under `tests/conformance` as filed (typically `tests/conformance/fixtures/es/annex-b`). Harness `tests/conformance/tests/annex_b.rs`. Target js. Not E17.02, S02 / E19.02, or N08.16.

## Verify

`cargo test -p draconic-conformance --test annex_b` prints `test result: ok.` Workspace `cargo test --workspace` stays green. Child (if split) is `done` on ROADMAP.md; E18.44 stays `todo` unless no remainder remains.

scope: `ROADMAP.md` (E18.44 / one new child), `tests/conformance/fixtures/es/annex-b`, `tests/conformance/tests/annex_b.rs`, js frontend/runtime as needed for that remainder

## Links

[[s-e18-44]] [[ticket-30-e18-44-untracked-ecma-262-remainder-beyond]]
