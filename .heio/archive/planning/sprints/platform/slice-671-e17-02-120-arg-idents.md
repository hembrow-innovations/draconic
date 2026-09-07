---
id: "slice-671-e17-02-120-arg-idents"
title: "E17.02.120 arg idents hit object"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-07T08:32:15Z"
updated_at: "2026-09-07T14:10:00Z"
---

# E17.02.120 arg idents hit object

## Why

Roadmap E17.02.120 is already `done` and claims string/method/arg idents hit the with object. The fixture never uses a with-object property as a call argument to `trimLeft` / `trimRight`. The only argument identifier is unresolvable `e1702120_arg`. This cut runs resolvable arg idents through the object environment.

## Done

The `es/legacy/with_string_trim_left_right` Program uses with-object properties as arguments to `trimLeft` / `trimRight` on the js target. Those idents hit the object, else outer. The legacy harness stays green. E17.02.120 stays `done`. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **Counting unresolvable `e1702120_arg` as arg-ident coverage**
- **Reopening E17.02.120** as a new language Loop atom
- **E17.02.121 accessor-legacy arg idents** ([[ticket-659-e17-02-121-arg-idents]])
- **N08.15**: native observations of non-strict legacy
- **Marking E17.02 done**
- **S02 / E19.02**: Test262 allowlist expansion
- **`cargo test --workspace` as this slice's oracle** (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [x] O1: with_string_trim_left_right still runs on declared js, including arg-ident observations
  CHECK: cargo test -p draconic-conformance --test legacy with_string_trim_left_right
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.10s. Program uses resolvable with-object properties as arguments to `trimLeft` / `trimRight` and asserts those idents hit the object, else outer.

## Pool

Durable links to task ids. Never drop them.

- `[[task-672-e17-02-120-arg-idents]]`

## See also

[[ticket-660-e17-02-120-arg-idents]] ROADMAP.md E17.02.120 [[Language purpose]] [[ECMA — Contract]] [[ticket-202-e17-02-non-strict-legacy]] [[ticket-659-e17-02-121-arg-idents]]
