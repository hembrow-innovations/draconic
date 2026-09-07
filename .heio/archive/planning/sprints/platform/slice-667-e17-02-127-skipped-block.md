---
id: "slice-667-e17-02-127-skipped-block"
title: "E17.02.127 skipped block not assigned through with"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-07T06:58:11Z"
updated_at: "2026-09-07T13:10:00Z"
---

# E17.02.127 skipped block not assigned through with

## Why

Roadmap E17.02.127 is already `done` and claims a skipped block-level function is not assigned through the object environment. Both skip tests wrap the entire `with` in `if (false)`, so the inner `{ function k(){…} }` never sits under a live with object environment. This cut skips the block inside `with`.

## Done

The `es/legacy/with_block_function` Program skips a block-level function declaration while `with` is on the stack on the js target. That name is not assigned. The legacy harness stays green. E17.02.127 stays `done`. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **Wrapping the whole `with` in `if (false)` as coverage**
- **Reopening E17.02.127** as a new language Loop atom
- **E17.02.126 if-function skip** (`with_if_function`): different child
- **N08.15**: native observations of non-strict legacy
- **Marking E17.02 done**
- **S02 / E19.02**: Test262 allowlist expansion
- **`cargo test --workspace` as this slice's oracle** (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [x] O1: with_block_function still runs on declared js, including skip-inside-with
  CHECK: cargo test -p draconic-conformance --test legacy with_block_function
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.09s. Program skips `{ function k(){…} }` inside `with` (not around it) and asserts `k` is not assigned on the enclosing VE and `obj` did not gain `k`.

## Pool

Durable links to task ids. Never drop them.

- `[[task-668-e17-02-127-skipped-block]]`

## See also

[[ticket-658-e17-02-127-skipped-block]] ROADMAP.md E17.02.127 [[Language purpose]] [[ECMA — Contract]] [[ticket-202-e17-02-non-strict-legacy]]
