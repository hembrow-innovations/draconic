---
id: "slice-581-e17-02-170-arguments-iterator"
title: "E17.02.170 arguments @@iterator residual without with"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T23:30:00Z"
---

# E17.02.170 arguments @@iterator residual without with

## Why

ROADMAP E17.02 stays the leftover non-strict legacy bucket. This cut files one discovered child: the `arguments` object's `@@iterator` residual without wrapping the program in `with`.

## Done

ROADMAP E17.02.170 is implemented test-first on the js target. Fixtures under `tests/conformance` `es/legacy` plus the `legacy` harness are green. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **E17.02.01–.22 / .19**: callee, mapped args, exotic for-of/spread, Array mutators already tracked
- **E17.01**: `with` statement basics
- **E18.44**: untracked ECMA-262 remainder outside this legacy bucket
- **S02 / E19.02**: Test262 allowlist expansion
- **N08.15**: native observations of non-strict legacy
- Marking E17.02 done
- `cargo test --workspace` as this slice's oracle (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [x] O1: E17.02.170 fixtures run on the declared js target through the legacy harness
  CHECK: cargo test -p draconic-conformance --test legacy
  EXPECT: test result: ok.
  EVIDENCE: cargo test -p draconic-conformance --test legacy → test result: ok. 344 passed; 0 failed (arguments_iterator_runs)

## Pool

Durable links to task ids. Never drop them.

- `[[task-582-e17-02-170-arguments-iterator]]`

## See also

ROADMAP.md E17.02 / E17.02.170, `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, CONTEXT.md, [[location-218-conformance]], [[ticket-202-e17-02-non-strict-legacy]].
