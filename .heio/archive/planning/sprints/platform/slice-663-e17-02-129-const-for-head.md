---
id: "slice-663-e17-02-129-const-for-head"
title: "E17.02.129 const for-head does not write object"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-07T06:28:34Z"
updated_at: "2026-09-07T13:00:00Z"
---
# E17.02.129 const for-head does not write object

## Why

Roadmap E17.02.129 is already `done` and claims `let`/`const` in a `for` head does not write the with object. The fixture runs `for (let …)` only. This cut locks `for (const …)` so that claim cannot stay green without running const heads.

## Done

The `es/legacy/with_var_for` Program runs `for (const …)` heads inside `with` on the js target. Those bindings do not write the with object. The legacy harness stays green. E17.02.129 stays `done`. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **Ticket 657**: accessor get `this` on this same row
- **Reopening E17.02.129** as a new language Loop atom
- **Classic `for (const i = 0; …; i = i + 1)`**: const reassignment, not this claim
- **N08.15**: native observations of non-strict legacy
- **Marking E17.02 done**
- **S02 / E19.02**: Test262 allowlist expansion
- **`cargo test --workspace` as this slice's oracle** (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [x] O1: with_var_for still runs on declared js, including const for-heads
  CHECK: cargo test -p draconic-conformance --test legacy with_var_for
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.09s. Program contains `for (const` for-in and for-of.

## Pool

Durable links to task ids. Never drop them.

- `[[task-664-e17-02-129-const-for-head]]`

## See also

[[ticket-656-e17-02-129-const-for-head]] ROADMAP.md E17.02.129 [[Language purpose]] [[ECMA — Contract]] [[ticket-202-e17-02-non-strict-legacy]] [[ticket-657-e17-02-129-accessor-get-this]]
