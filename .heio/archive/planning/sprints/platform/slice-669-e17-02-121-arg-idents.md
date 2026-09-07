---
id: "slice-669-e17-02-121-arg-idents"
title: "E17.02.121 arg idents hit object"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-07T07:17:00Z"
updated_at: "2026-09-07T17:45:00Z"
---

# E17.02.121 arg idents hit object

## Why

Roadmap E17.02.121 is already `done` and claims object/method/arg idents hit the with object. The fixture never uses a with-object property as a call argument to `__defineGetter__` / `__defineSetter__` / `__lookupGetter__` / `__lookupSetter__`. The only argument identifier is unresolvable `e1702121_arg`. This cut runs resolvable arg idents through the object environment.

## Done

The `es/legacy/with_object_accessor_legacy` Program uses with-object properties as arguments to those four methods on the js target. Those idents hit the object, else outer. The legacy harness stays green. E17.02.121 stays `done`. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **Counting unresolvable `e1702121_arg` as arg-ident coverage**
- **Reopening E17.02.121** as a new language Loop atom
- **E17.02.120 trimLeft/trimRight arg idents** ([[ticket-660-e17-02-120-arg-idents]])
- **N08.15**: native observations of non-strict legacy
- **Marking E17.02 done**
- **S02 / E19.02**: Test262 allowlist expansion
- **`cargo test --workspace` as this slice's oracle** (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [x] O1: with_object_accessor_legacy still runs on declared js, including arg-ident observations
  CHECK: cargo test -p draconic-conformance --test legacy with_object_accessor_legacy
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.09s. Program uses resolvable with-object properties as arguments to `__defineGetter__` / `__defineSetter__` / `__lookupGetter__` / `__lookupSetter__` and asserts those idents hit the object, else outer.

## Pool

Durable links to task ids. Never drop them.

- `[[task-670-e17-02-121-arg-idents]]`

## See also

[[ticket-659-e17-02-121-arg-idents]] ROADMAP.md E17.02.121 [[Language purpose]] [[ECMA — Contract]] [[ticket-202-e17-02-non-strict-legacy]] [[ticket-660-e17-02-120-arg-idents]]
