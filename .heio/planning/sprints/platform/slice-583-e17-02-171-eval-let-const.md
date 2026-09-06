---
id: "slice-583-e17-02-171-eval-let-const"
title: "E17.02.171 direct eval of let/const without caller inject"
kind: slice
status: active
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T23:45:00Z"
---
# E17.02.171 direct eval of let/const without caller inject

## Why

ROADMAP E17.02 stays the leftover non-strict legacy bucket. This cut files one discovered child: direct eval of `let`/`const` does not inject into the caller's VariableEnvironment.

## Done

ROADMAP E17.02.171 is implemented test-first on the js target. Fixtures under `tests/conformance` `es/legacy` plus the `legacy` harness are green. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **E17.02.11 / .26 / .29**: var/function inject, `"use strict"` source, delete of injected var
- **E16.01–.03**: eval / Function / indirect eval basics
- **E17.01**: `with` statement basics
- **E18.44**: untracked ECMA-262 remainder outside this legacy bucket
- **S02 / E19.02**: Test262 allowlist expansion
- **N08.15**: native observations of non-strict legacy
- Marking E17.02 done
- `cargo test --workspace` as this slice's oracle (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [x] O1: E17.02.171 fixtures run on the declared js target through the legacy harness
  CHECK: cargo test -p draconic-conformance --test legacy
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 346 passed; 0 failed (eval_let_const_runs)

## Pool

Durable links to task ids. Never drop them.

- `[[task-584-e17-02-171-eval-let-const]]`

## See also

ROADMAP.md E17.02 / E17.02.171, `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, CONTEXT.md, [[location-218-conformance]], [[ticket-202-e17-02-non-strict-legacy]].
