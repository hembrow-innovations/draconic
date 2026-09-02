---
id: "e17-02-168-assign-update-target"
title: "E17.02.168 with + assignment/update target residual"
kind: task
status: completed
tags: []
created_at: "2026-09-02T19:32:00Z"
updated_at: "2026-09-02T19:40:00Z"
---

# E17.02.168 with + assignment/update target residual

## Done

One atomic untracked remainder of ROADMAP E17.02 is implemented test-first on the js target: `with` + assignment/update target residual (E19.60) through object environment; `tests/conformance/fixtures/es/legacy` plus the `legacy` harness are green for that remainder.

## Context

Roadmap ID **E17.02** (other non-strict legacy beyond `with`). Tracked E17.02 children stay `done`. This sitting implements E17.02.168: parenthesized cover id `(id)++`/`(id)=`; const PutValue / dstr put-const runtime TypeError not compile reject; non-strict `eval`/`arguments` assignment target, all through `with` object environment. Leave E17.02 `todo` while untracked remainder remains. Fixtures under `tests/conformance/fixtures/es/legacy`. Harness `tests/conformance/tests/legacy.rs`. Target js. Not E17.01, E18.44, N08.15, or Test262 full allowlist.

## Verify

`cargo test -p draconic-conformance --test legacy` prints `test result: ok.` Workspace `cargo test --workspace` stays green. Child E17.02.168 is `done` on ROADMAP.md; E17.02 stays `todo`.

scope: `ROADMAP.md` (E17.02 / E17.02.168), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, js frontend as needed for that remainder

## Links

[[s-e17-02-remainder]] [[ticket-29-e17-02-non-strict-legacy]]
