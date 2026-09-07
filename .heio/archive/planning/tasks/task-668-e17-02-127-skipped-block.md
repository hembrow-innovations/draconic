---
id: "task-668-e17-02-127-skipped-block"
title: "Lock skipped block-level function so it is not assigned through with"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-667-e17-02-127-skipped-block"
tags: []
created_at: "2026-09-07T06:58:11Z"
updated_at: "2026-09-07T13:00:00Z"
---

# Lock skipped block-level function so it is not assigned through with

## Blocked by

None.

## Done

`es/legacy/with_block_function` skips `{ function k(){…} }` while `with` is on the stack on js. That name is not assigned. `cargo test -p draconic-conformance --test legacy with_block_function` prints `test result: ok.` E17.02.127 stays `done`.

## Context

Roadmap E17.02.127 already claims a skipped Annex B.3.3 block-level function is not assigned through the object environment. `skippedBlockNotAssigned` and `skippedOuterIfNotAssigned` both wrap the entire `with` in `if (false)`, so the inner block is never skipped while an object environment is live. Contrast `with_if_function.drac` `skippedBranchNotAssigned`, which skips inside `with`. Add a skip-inside-with case in the same fixture style: `with (obj) { if (false) { { function k(){…} } } }`, then assert `k` is not assigned on the enclosing VE and `obj` did not gain `k`. Wrapping the whole `with` in `if (false)` does not count. Leave E17.02 `todo`. Native observations are N08.15.

CHECK timeout for O1 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes.

## Verify

`cargo test -p draconic-conformance --test legacy with_block_function` prints `test result: ok.` The running Program skips a block-level function inside `with`. Slice O1 EVIDENCE is filled. E17.02.127 remains `done`; E17.02 remains `todo`.

scope: `tests/conformance/fixtures/es/legacy` (`with_block_function` Program and meta), `tests/conformance/tests/legacy.rs` only if the harness needs a new title, `docs/specs/draconic/language/ecma/` if the tests map should name the skip-inside-with case, js emit only if the new case fails, [[slice-667-e17-02-127-skipped-block]] EVIDENCE

## Links

[[slice-667-e17-02-127-skipped-block]] [[ticket-658-e17-02-127-skipped-block]]

## Agent Brief

**Category:** bug
**Summary:** Lock E17.02.127’s skipped-block claim by skipping `{ function k(){…} }` while `with` is on the stack so a skip that never enters `with` cannot stay green.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

**Skills:** load **tdd**, **draconic-loop**, **draconic-language**, **rust-development**, **docs**, **spec**. This is a fixture lock on a `done` row, not a new Roadmap atom.

**Intent:**
- Promise ids: `language.ecma:legacy-with`
- Purpose: [[Language purpose]]
- Contract-first: assert the existing legacy-with promise, extend the `with_block_function` observations, then emit only if those cases fail
- Product behaviour should already match. This sitting makes the claimed skip-inside-with unskippable

**Current behavior:**
`with` plus Annex B.3.3 block-level functions is locked. Both skip tests wrap the entire `with` in `if (false)`. The inner `{ function k(){…} }` never runs under a live object environment. `with_if_function` already skips inside `with` for a different child.

**Desired behavior:**
Inside `with (obj)`, a skipped `{ function k(){…} }` does not assign `k` onto the enclosing VE or onto `obj`. Existing assigned-when-block-runs and other observations stay true. The fixture meta checks the new observation. js target only.

**Key interfaces:**
- Object environment inside `with`: Annex B.3.3 instantiate-and-assign runs when the block evaluates, not when `with` is merely present
- Conformance `.drac` + `.meta` for `es/legacy/with_block_function`, harness `legacy` present and runs tests
- `language.ecma:legacy-with` already locked at `with` basics; this sitting tightens the filed E17.02.127 child, not a new promise id

**Acceptance criteria:**
- [x] The `with_block_function` Program skips `{ function k(){…} }` inside `with`, not around it
- [x] The skipped name is not assigned on the enclosing VE
- [x] The with object does not gain that name
- [x] Existing assigned-when-block-runs observations still pass
- [x] `cargo test -p draconic-conformance --test legacy with_block_function` prints `test result: ok.`
- [x] E17.02.127 remains `done`; E17.02 remains `todo`
- [x] Slice O1 EVIDENCE is filled

**Out of scope:**
- Counting `if (false) { with (…) { … } }` as coverage
- E17.02.126 if-function skip (`with_if_function`)
- Native observations (N08.15)
- Marking E17.02 done
- Test262 allowlist expansion
- Workspace CHECK as this task’s oracle
- Respect [[Language purpose]] Out of scope fences

## Gauntlet

- **round:** 1
- **command:** cargo test -p draconic-conformance --test legacy with_block_function
- **result:** win
- **gap:** none. test result: ok. 2 passed. Program skips `{ function k(){…} }` inside `with` and asserts the name is unassigned on the enclosing VE and on `obj`.
