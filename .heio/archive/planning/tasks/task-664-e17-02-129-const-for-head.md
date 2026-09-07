---
id: "task-664-e17-02-129-const-for-head"
title: "Lock const for-heads so they do not write the with object"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-663-e17-02-129-const-for-head"
tags: []
created_at: "2026-09-07T06:28:34Z"
updated_at: "2026-09-07T12:45:00Z"
---
# Lock const for-heads so they do not write the with object

## Blocked by

None.

## Done

`es/legacy/with_var_for` runs `for (const …)` inside `with` on js. Those bindings do not write the with object. `cargo test -p draconic-conformance --test legacy with_var_for` prints `test result: ok.` E17.02.129 stays `done`.

## Context

Roadmap E17.02.129 already claims `let`/`const` in a `for` head does not write the with object. The existing Program runs `for (let k in …)` and classic `for (let i = …)`. It never runs `for (const …)`, and no other `es/legacy` fixture does either. Add const for-in and const for-of cases in the same fixture style: inside `with (obj)`, iterate, then assert `obj` did not gain or change the loop binding. Wire those observations into the fixture meta checks. Do not write `for (const i = 0; …; i = i + 1)` — that is const reassignment, not this claim. Leave accessor get `this` to [[ticket-657-e17-02-129-accessor-get-this]]. Leave E17.02 `todo`. Native observations are N08.15.

CHECK timeout for O1 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes.

## Verify

`cargo test -p draconic-conformance --test legacy with_var_for` prints `test result: ok.` The running Program contains `for (const` heads. Slice O1 EVIDENCE is filled. E17.02.129 remains `done`; E17.02 remains `todo`.

scope: `tests/conformance/fixtures/es/legacy` (`with_var_for` Program and meta), `tests/conformance/tests/legacy.rs` only if the harness needs a new title, `docs/specs/draconic/language/ecma/` if the tests map should name the const cases, js emit only if the new cases fail, [[slice-663-e17-02-129-const-for-head]] EVIDENCE

## Links

[[slice-663-e17-02-129-const-for-head]] [[ticket-656-e17-02-129-const-for-head]]

## Agent Brief

**Category:** bug
**Summary:** Lock E17.02.129’s `const` for-head claim by actually running `for (const …)` inside `with` so a missing const path cannot stay green.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

**Skills:** load **tdd**, **draconic-loop**, **draconic-language**, **rust-development**, **docs**, **spec**. This is a fixture lock on a `done` row, not a new Roadmap atom.

**Intent:**
- Promise ids: `language.ecma:legacy-with`
- Purpose: [[Language purpose]]
- Contract-first: assert the existing legacy-with promise, extend the `with_var_for` observations, then emit only if those cases fail
- Product behaviour should already match. This sitting makes the claimed const for-head unskippable

**Current behavior:**
`with` plus `var` in `for` heads is locked. `let` for-in and classic `let` for-heads assert they do not write the with object. `const` for-heads are named on E17.02.129 and never executed.

**Desired behavior:**
Inside `with (obj)`, `for (const k in …)` and `for (const v of …)` iterate without assigning `k` or `v` onto `obj`. Existing `let` and `var` observations stay true. The fixture meta checks the new observations. js target only.

**Key interfaces:**
- Object environment inside `with`: lexical `const` for-head bindings must not HasBinding-assign onto the with object
- Conformance `.drac` + `.meta` for `es/legacy/with_var_for`, harness `legacy` present and runs tests
- `language.ecma:legacy-with` already locked at `with` basics; this sitting tightens the filed E17.02.129 child, not a new promise id

**Acceptance criteria:**
- [x] The `with_var_for` Program contains and runs `for (const` heads inside `with`
- [x] Const for-in does not write the with object’s matching property
- [x] Const for-of does not write the with object’s matching property
- [x] Existing `let` / `var` observations still pass
- [x] `cargo test -p draconic-conformance --test legacy with_var_for` prints `test result: ok.`
- [x] E17.02.129 remains `done`; E17.02 remains `todo`
- [x] Slice O1 EVIDENCE is filled

**Out of scope:**
- Accessor get `this` ([[ticket-657-e17-02-129-accessor-get-this]])
- Classic `for (const i = 0; …; i = i + 1)` (const reassignment)
- Native observations (N08.15)
- Marking E17.02 done
- Test262 allowlist expansion
- Workspace CHECK as this task’s oracle
- Respect [[Language purpose]] Out of scope fences

## Gauntlet

- **round:** 1
- **command:** cargo test -p draconic-conformance --test legacy with_var_for
- **result:** win
- **gap:** none. test result: ok. 2 passed. Program contains `for (const` for-in and for-of.
