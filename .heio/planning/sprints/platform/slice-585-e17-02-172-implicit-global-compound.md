---
id: "slice-585-e17-02-172-implicit-global-compound"
title: "E17.02.172 unresolvable compound/update does not create a global"
kind: slice
status: frozen
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T11:49:42Z"
---

# E17.02.172 unresolvable compound/update does not create a global

## Why

ROADMAP E17.02 stays the leftover non-strict legacy bucket. This cut files one discovered child: compound assignment and update on an unresolvable identifier GetValue-first throw and must not create an implicit global.

## Done

ROADMAP E17.02.172 is implemented test-first on the js target. Fixtures under `tests/conformance` `es/legacy` plus the `legacy` harness are green. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **E17.02.07 / .09 / .16 / .66**: simple implicit globals, for-in/of, immutable-global compound, with-only unresolvable LHS
- **E17.01**: `with` statement basics
- **E18.44**: untracked ECMA-262 remainder outside this legacy bucket
- **S02 / E19.02**: Test262 allowlist expansion
- **N08.15**: native observations of non-strict legacy
- Marking E17.02 done
- `cargo test --workspace` as this slice's oracle (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [ ] O1: E17.02.172 fixtures run on the declared js target through the legacy harness
  CHECK: cargo test -p draconic-conformance --test legacy
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- `[[task-586-e17-02-172-implicit-global-compound]]`

## See also

ROADMAP.md E17.02 / E17.02.172, `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, CONTEXT.md, [[location-218-conformance]], [[ticket-202-e17-02-non-strict-legacy]].
