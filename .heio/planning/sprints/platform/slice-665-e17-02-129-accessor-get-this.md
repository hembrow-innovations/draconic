---
id: "slice-665-e17-02-129-accessor-get-this"
title: "E17.02.129 accessor get this is with object"
kind: slice
status: active
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-07T06:34:01Z"
updated_at: "2026-09-07T23:59:00Z"
---

# E17.02.129 accessor get this is with object

## Why

Roadmap E17.02.129 is already `done` and claims accessor get/set `this` is the with object. The fixture asserts setter `this` only. Getters on those objects are never invoked, so a wrong getter `this` still passes. This cut locks getter `this` by actually reading.

## Done

The `es/legacy/with_var_for` Program invokes getters inside `with` on the js target and asserts getter `this` is the with object. The legacy harness stays green. E17.02.129 stays `done`. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **Ticket 656 / slice 663**: const for-head does not write object
- **Reopening E17.02.129** as a new language Loop atom
- **Counting unused `get k()` on setter objects as coverage**
- **N08.15**: native observations of non-strict legacy
- **Marking E17.02 done**
- **S02 / E19.02**: Test262 allowlist expansion
- **`cargo test --workspace` as this slice's oracle** (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [x] O1: with_var_for still runs on declared js, including getter-this observations
  CHECK: cargo test -p draconic-conformance --test legacy with_var_for
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.09s. Program invokes for-in/for-of right-ident and Annex B init-RHS getters and asserts getter this is the with object.

## Pool

Durable links to task ids. Never drop them.

- `[[task-666-e17-02-129-accessor-get-this]]`

## See also

[[ticket-657-e17-02-129-accessor-get-this]] ROADMAP.md E17.02.129 [[Language purpose]] [[ECMA — Contract]] [[ticket-202-e17-02-non-strict-legacy]] [[ticket-656-e17-02-129-const-for-head]] [[slice-663-e17-02-129-const-for-head]]
