---
id: "task-586-e17-02-172-implicit-global-compound"
title: "E17.02.172 unresolvable compound/update does not create a global"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-585-e17-02-172-implicit-global-compound"
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T19:15:00Z"
---

# E17.02.172 unresolvable compound/update does not create a global

## Blocked by

None.

## Done

ROADMAP E17.02.172 is green on the js target: unresolvable `+=` / `++` / `--` throw ReferenceError and do not create `globalThis` properties; already-created implicit globals still update.

## Context

Roadmap ID **E17.02.172** (child of E17.02). E17.02.07 locked simple `=` implicit globals. E17.02.66 locked GetValue-first ReferenceError only through `with`. This sitting files the non-with compound/update case. Leave E17.02 `todo`. Not E17.01, E18.44, N08.15, or Test262 full.

CHECK timeout for O1 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes. If review adds a workspace CHECK, set `ORACLE_CHECK_TIMEOUT_MS=900000`.

## Verify

`cargo test -p draconic-conformance --test legacy` prints `test result: ok.` E17.02.172 is `done` on ROADMAP.md; E17.02 stays `todo`.

scope: `ROADMAP.md` (E17.02.172), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, js assignment emit as needed

## Links

[[slice-585-e17-02-172-implicit-global-compound]] [[ticket-202-e17-02-non-strict-legacy]]

## Agent Brief

**Category:** enhancement
**Summary:** Lock non-strict unresolvable compound and update so they throw and do not create implicit globals.

**Intent (required when product behaviour changes):**
- Promise ids: `language.ecma:expressions`
- Purpose: [[docs/specs/draconic/language/ecma/contract]]
- Contract-first: assert promise → test → code

**Current behavior:**
E17.02.07 only covers simple equals on a free identifier, which creates a global property. E17.02.16 covers plus-equals and plus-plus on existing immutable globals. E17.02.66 covers GetValue-first ReferenceError only through `with`. `implicit_global.drac` has no plus-equals or plus-plus.

**Desired behavior:**
In non-strict code, `missing += 1`, `++missing`, `missing++`, and `--missing` throw ReferenceError, do not create `globalThis.missing`, and do not evaluate the compound right-hand side. An already-created implicit global still updates under plus-plus and plus-equals. Strict-mode unresolvable compound and update stay ReferenceError.

**Key interfaces:**
- PutValue / GetValue on unresolvable IdentifierReference for compound and update
- Conformance `.drac` + `.meta` under es/legacy, harness `legacy` present and runs tests

**Acceptance criteria:**
- [x] unresolvable `+=` / `++` / `--` throw ReferenceError and do not create `globalThis.missing`
- [x] compound RHS is not evaluated when the identifier is unresolvable
- [x] an already-created implicit global still updates
- [x] `cargo test -p draconic-conformance --test legacy` prints `test result: ok.`
- [x] E17.02.172 is `done`; E17.02 stays `todo`

**Out of scope:**
- Logical assignment
- for-await and destructuring unresolvable targets
- preventExtensions on globalThis
- Native observations
- Marking E17.02 done
- Workspace CHECK as this task's oracle

## Gauntlet

- **round:** 1
- **command:** cargo test -p draconic-conformance --test legacy
- **result:** win
- **gap:** none. Fixture locks GetValue-first `language.ecma:expressions` on js; emit already uses native `+=` / `++` / `--`.
