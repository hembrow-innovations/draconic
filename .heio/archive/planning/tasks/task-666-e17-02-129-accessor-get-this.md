---
id: "task-666-e17-02-129-accessor-get-this"
title: "Lock accessor get this on for-heads so it is the with object"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-665-e17-02-129-accessor-get-this"
tags: []
created_at: "2026-09-07T06:34:01Z"
updated_at: "2026-09-07T12:50:00Z"
---

# Lock accessor get this on for-heads so it is the with object

## Blocked by

None.

## Done

`es/legacy/with_var_for` invokes getters inside `with` on js and asserts getter `this` is the with object. `cargo test -p draconic-conformance --test legacy with_var_for` prints `test result: ok.` E17.02.129 stays `done`.

## Context

Roadmap E17.02.129 already claims accessor get/set `this` is the with object. The existing Program records setter `this` in `forInAccessorSetThisIsWithObject`, `annexBAccessorSetThisIsWithObject`, and `forOfAccessorSetThisIsWithObject`. Those objects also define getters that only return `this._k` / `this._v` and are never called, so a wrong getter `this` still passes. Sibling `with_var_decl.drac` already runs `accessorGetThisIsWithObject` (`var x = y` inside `with`). Add getter-this cases in the same fixture style: inside `with (obj)`, invoke a getter (right ident of for-in/for-of, and Annex B initializer RHS), record `this`, assert `seen === obj`. Unused getters on the setter objects do not count. Leave const for-heads to [[ticket-656-e17-02-129-const-for-head]]. Leave E17.02 `todo`. Native observations are N08.15.

CHECK timeout for O1 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes.

## Verify

`cargo test -p draconic-conformance --test legacy with_var_for` prints `test result: ok.` The running Program invokes getters and asserts getter `this` is the with object. Slice O1 EVIDENCE is filled. E17.02.129 remains `done`; E17.02 remains `todo`.

scope: `tests/conformance/fixtures/es/legacy` (`with_var_for` Program and meta), `tests/conformance/tests/legacy.rs` only if the harness needs a new title, `docs/specs/draconic/language/ecma/` if the tests map should name the getter-this cases, js emit only if the new cases fail, [[slice-665-e17-02-129-accessor-get-this]] EVIDENCE

## Links

[[slice-665-e17-02-129-accessor-get-this]] [[ticket-657-e17-02-129-accessor-get-this]]

## Agent Brief

**Category:** bug
**Summary:** Lock E17.02.129’s accessor get `this` claim by actually invoking getters inside `with` so a wrong getter `this` cannot stay green.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

**Skills:** load **tdd**, **draconic-loop**, **draconic-language**, **rust-development**, **docs**, **spec**. This is a fixture lock on a `done` row, not a new Roadmap atom.

**Intent:**
- Promise ids: `language.ecma:legacy-with`
- Purpose: [[Language purpose]]
- Contract-first: assert the existing legacy-with promise, extend the `with_var_for` observations, then emit only if those cases fail
- Product behaviour should already match. This sitting makes the claimed getter `this` unskippable

**Current behavior:**
`with` plus `var` in `for` heads is locked. Setter `this` is asserted on for-in, Annex B for-in, and for-of. Getters on those objects are never invoked. Sibling `es/legacy/with_var_decl` already reads a getter.

**Desired behavior:**
Inside `with (obj)`, for-in right ident, for-of right ident, and Annex B initializer RHS invoke getters whose `this` is `obj`. Existing setter `this` and `let` / `var` observations stay true. The fixture meta checks the new observations. js target only.

**Key interfaces:**
- Object environment inside `with`: getter `this` on HasBinding reads must be the with object
- Conformance `.drac` + `.meta` for `es/legacy/with_var_for`, harness `legacy` present and runs tests
- `language.ecma:legacy-with` already locked at `with` basics; this sitting tightens the filed E17.02.129 child, not a new promise id

**Acceptance criteria:**
- [x] The `with_var_for` Program contains and runs getter-this cases inside `with`
- [x] For-in right ident getter `this` is the with object
- [x] For-of right ident getter `this` is the with object
- [x] Annex B initializer RHS getter `this` is the with object
- [x] Existing setter `this` / `let` / `var` observations still pass
- [x] `cargo test -p draconic-conformance --test legacy with_var_for` prints `test result: ok.`
- [x] E17.02.129 remains `done`; E17.02 remains `todo`
- [x] Slice O1 EVIDENCE is filled

**Out of scope:**
- Const for-heads ([[ticket-656-e17-02-129-const-for-head]])
- Counting unused `get k()` on setter objects as coverage
- Native observations (N08.15)
- Marking E17.02 done
- Test262 allowlist expansion
- Workspace CHECK as this task’s oracle
- Respect [[Language purpose]] Out of scope fences
