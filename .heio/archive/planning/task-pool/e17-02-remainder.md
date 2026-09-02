---
id: "e17-02-remainder"
title: "E17.02 one atomic non-strict legacy remainder"
kind: task
status: completed
tags: []
created_at: "2026-09-02T08:55:23Z"
updated_at: "2026-09-02T09:30:00Z"
---

# E17.02 one atomic non-strict legacy remainder

## Done

One atomic untracked remainder of ROADMAP E17.02 is implemented test-first on the js target; `tests/conformance/fixtures/es/legacy` plus the `legacy` harness are green for that remainder.

## Context

Roadmap ID **E17.02** (other non-strict legacy beyond `with`). Tracked E17.02 children stay `done`. This sitting implements one untracked remainder (sloppy-mode Annex B / arguments / eval / PutValue gaps). If E17.02 is larger than one sitting, split one child under E17.02 and complete only that child; leave E17.02 `todo` while untracked remainder remains. Mark E17.02 `done` only when no untracked remainder stays. Fixtures under `tests/conformance/fixtures/es/legacy`. Harness `tests/conformance/tests/legacy.rs`. Target js. Not E17.01, E18.44, N08.15, or Test262 full allowlist.

## Verify

`cargo test -p draconic-conformance --test legacy` prints `test result: ok.` Workspace `cargo test --workspace` stays green. Child (if split) is `done` on ROADMAP.md; E17.02 stays `todo` unless no remainder remains.

scope: `ROADMAP.md` (E17.02 / one new child), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, js frontend/runtime as needed for that remainder

## Links

[[s-e17-02]] [[ticket-27-e17-02-non-strict-legacy]]
