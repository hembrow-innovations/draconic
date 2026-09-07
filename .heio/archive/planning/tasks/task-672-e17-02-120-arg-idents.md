---
id: "task-672-e17-02-120-arg-idents"
title: "Lock trimLeft/trimRight arg idents so they hit the with object"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-671-e17-02-120-arg-idents"
tags: []
created_at: "2026-09-07T08:32:15Z"
updated_at: "2026-09-07T14:00:00Z"
---

# Lock trimLeft/trimRight arg idents so they hit the with object

## Blocked by

None.

## Done

`es/legacy/with_string_trim_left_right` uses with-object properties as arguments to `trimLeft` / `trimRight` on js. Those idents hit the object, else outer. `cargo test -p draconic-conformance --test legacy with_string_trim_left_right` prints `test result: ok.` E17.02.120 stays `done`.

## Context

Roadmap E17.02.120 already claims string/method/arg idents hit the with object. The existing Program calls `trimLeft` / `trimRight` with no args or unresolvable `e1702120_arg`. A skip of the object environment for args would still pass. Contrast `with_regexp_compile.drac` `argsHitObject`. Add args-hit-object and args-miss-uses-outer cases in the same fixture style for both methods. The methods may ignore extra arguments; the ident still must resolve. Unresolvable `e1702120_arg` does not count. Leave accessor-legacy args to [[ticket-659-e17-02-121-arg-idents]]. Leave E17.02 `todo`. Native observations are N08.15.

CHECK timeout for O1 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes.

## Verify

`cargo test -p draconic-conformance --test legacy with_string_trim_left_right` prints `test result: ok.` The running Program uses resolvable with-object properties as those method arguments. Slice O1 EVIDENCE is filled. E17.02.120 remains `done`; E17.02 remains `todo`.

scope: `tests/conformance/fixtures/es/legacy` (`with_string_trim_left_right` Program and meta), `tests/conformance/tests/legacy.rs` only if the harness needs a new title, `docs/specs/draconic/language/ecma/` if the tests map should name the arg-ident cases, js emit only if the new cases fail, [[slice-671-e17-02-120-arg-idents]] EVIDENCE

## Links

[[slice-671-e17-02-120-arg-idents]] [[ticket-660-e17-02-120-arg-idents]]

## Agent Brief

**Category:** bug
**Summary:** Lock E17.02.120’s arg-ident claim by passing with-object properties into `trimLeft` / `trimRight` so a skip of args cannot stay green.

**Drain:** `/afk-task`. Unblocked. Claim `status: ready` then `mode: afk`.

**Skills:** load **tdd**, **draconic-loop**, **draconic-language**, **rust-development**, **docs**, **spec**. This is a fixture lock on a `done` row, not a new Roadmap atom.

**Intent:**
- Promise ids: `language.ecma:legacy-with`
- Purpose: [[Language purpose]]
- Contract-first: assert the existing legacy-with promise, extend the `with_string_trim_left_right` observations, then emit only if those cases fail
- Product behaviour should already match. This sitting makes the claimed arg idents unskippable

**Current behavior:**
`with` plus `trimLeft` / `trimRight` is locked. String and method idents are observed. Arguments are absent or unresolvable `e1702120_arg`. Sibling `with_regexp_compile` already runs `argsHitObject`.

**Desired behavior:**
Inside `with (env)`, arguments to `trimLeft` / `trimRight` resolve on `env` when present, else outer. Existing unresolvable-arg and method-ident observations stay true. The fixture meta checks the new observations. js target only.

**Key interfaces:**
- Object environment inside `with`: argument IdentifierReferences use HasBinding on the with object
- Conformance `.drac` + `.meta` for `es/legacy/with_string_trim_left_right`, harness `legacy` present and runs tests
- `language.ecma:legacy-with` already locked at `with` basics; this sitting tightens the filed E17.02.120 child, not a new promise id

**Acceptance criteria:**
- [x] The `with_string_trim_left_right` Program uses resolvable with-object properties as arguments to `trimLeft` and `trimRight`
- [x] Arg idents hit the with object when present
- [x] Arg idents miss to outer when absent
- [x] Existing unresolvable-arg and method-ident observations still pass
- [x] `cargo test -p draconic-conformance --test legacy with_string_trim_left_right` prints `test result: ok.`
- [x] E17.02.120 remains `done`; E17.02 remains `todo`
- [x] Slice O1 EVIDENCE is filled

## Gauntlet

- **round 1**: `cargo test -p draconic-conformance --test legacy with_string_trim_left_right` — win. `test result: ok.` 2 passed. Diff vs `language.ecma:legacy-with`: arg IdentifierReferences to `trimLeft` / `trimRight` HasBinding on the with object, else outer. No purpose-fence product rules invented.

**Out of scope:**
- Counting unresolvable `e1702120_arg` as coverage
- E17.02.121 accessor-legacy arg idents ([[ticket-659-e17-02-121-arg-idents]])
- Native observations (N08.15)
- Marking E17.02 done
- Test262 allowlist expansion
- Workspace CHECK as this task’s oracle
- Respect [[Language purpose]] Out of scope fences
