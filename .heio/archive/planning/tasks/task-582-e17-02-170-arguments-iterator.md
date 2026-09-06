---
id: "task-582-e17-02-170-arguments-iterator"
title: "E17.02.170 arguments @@iterator residual without with"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-581-e17-02-170-arguments-iterator"
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T23:20:00Z"
---

# E17.02.170 arguments @@iterator residual without with

## Blocked by

None.

## Done

ROADMAP E17.02.170 is green on the js target: `arguments[Symbol.iterator]` is `Array.prototype.values`, descriptor residual, live mapped iteration, delete then spread/for-of TypeError, unmapped non-simple params keep the iterator.

## Context

Roadmap ID **E17.02.170** (child of E17.02). E17.02.19 locks for-of and spread consumption plus callee/length descriptors, not `@@iterator`. Leave E17.02 `todo`. Not E17.01, E18.44, N08.15, or Test262 full.

CHECK timeout for O1 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes. If review adds a workspace CHECK, set `ORACLE_CHECK_TIMEOUT_MS=900000`.

## Verify

`cargo test -p draconic-conformance --test legacy` prints `test result: ok.` E17.02.170 is `done` on ROADMAP.md; E17.02 stays `todo`.

scope: `ROADMAP.md` (E17.02.170), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, js function emit as needed

## Links

[[slice-581-e17-02-170-arguments-iterator]] [[ticket-202-e17-02-non-strict-legacy]]

## Agent Brief

**Category:** enhancement
**Summary:** Lock non-strict `arguments` `@@iterator` residual on js without `with`.

**Intent (required when product behaviour changes):**
- Promise ids: `language.ecma:functions`
- Purpose: [[docs/specs/draconic/language/ecma/contract]]
- Contract-first: assert promise → test → code

**Current behavior:**
E17.02.19 proves for-of and spread consume an arguments object and pins callee and length descriptors. It never reads `Symbol.iterator`, never checks identity with `Array.prototype.values`, never checks that iterator descriptor, and never deletes the iterator or iterates after a live mapped length change. `arguments_reflect_iter` is Reflect plus Array iteration methods, not `@@iterator`.

**Desired behavior:**
`arguments[Symbol.iterator]` is `Array.prototype.values`. The descriptor is writable, not enumerable, configurable. The iterator sees live mapped values and length. Delete of the iterator then spread or for-of is TypeError. Unmapped non-simple params keep the same iterator. Include named function expression and method.

**Key interfaces:**
- CreateMappedArgumentsObject / CreateUnmappedArgumentsObject iterator property
- Conformance `.drac` + `.meta` under es/legacy, harness `legacy` present and runs tests

**Acceptance criteria:**
- [x] `arguments[Symbol.iterator] === Array.prototype.values`
- [x] iterator descriptor is writable, non-enumerable, configurable
- [x] delete then spread or for-of is TypeError
- [x] `cargo test -p draconic-conformance --test legacy` prints `test result: ok.`
- [x] E17.02.170 is `done`; E17.02 stays `todo`

**Out of scope:**
- Re-filing E17.02.01 through .22
- Proxy on arguments
- `arguments.caller` versus `Function.caller`
- Native observations
- Marking E17.02 done
- Workspace CHECK as this task's oracle

## Gauntlet

- **round 1**: `cargo test -p draconic-conformance --test legacy` — win. `test result: ok.` 344 passed. Diff keeps `language.ecma:functions`; no `with`; js-only; E17.02 stays todo; identity/descriptor/live map/delete TypeError/unmapped/named FE+method locked.
