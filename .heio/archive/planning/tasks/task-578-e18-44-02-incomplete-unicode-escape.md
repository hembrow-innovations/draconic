---
id: "task-578-e18-44-02-incomplete-unicode-escape"
title: "E18.44.02 Annex B.1.4 incomplete UnicodeEscape IdentityEscape"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-577-e18-44-02-incomplete-unicode-escape"
tags: []
created_at: "2026-09-06T11:31:00Z"
updated_at: "2026-09-06T11:40:35Z"
---

# E18.44.02 Annex B.1.4 incomplete UnicodeEscape IdentityEscape

## Blocked by

None.

## Done

ROADMAP E18.44.02 is green on the js target: incomplete `\u` IdentityEscape without `u`/`v`, early SyntaxError with those flags, complete `\uXXXX` still a UnicodeEscape.

## Context

Roadmap ID **E18.44.02** (child of E18.44). E18.44.01 locked legacy octal / incomplete hex `\x`. This sitting files incomplete UnicodeEscape `\u` (not four hex digits) as IdentityEscape without `u`/`v`, and early SyntaxError with `u`/`v`. Leave E18.44 `todo`. Not E17.02, S02 / E19.02, or N08.16.

## Verify

`cargo test -p draconic-conformance --test annex_b` prints `test result: ok.` `cargo test -p draconic-lexer --lib` prints `test result: ok.` E18.44.02 is `done` on ROADMAP.md; E18.44 stays `todo`.

scope: `ROADMAP.md` (E18.44.02), `tests/conformance/fixtures/es/annex-b`, `tests/conformance/tests/annex_b.rs`, `crates/draconic-lexer` regexp validation as needed

## Links

[[slice-577-e18-44-02-incomplete-unicode-escape]] [[ticket-203-e18-44-untracked-ecma-262-remainder-beyond]]

## Agent Brief

**Category:** enhancement
**Summary:** Lock Annex B.1.4 incomplete `\u` IdentityEscape on js, including early SyntaxError under `u` and `v`.

**Intent (required when product behaviour changes):**
- Promise ids: `language.ecma:annex-b`
- Purpose: [[docs/specs/draconic/language/ecma/purpose]]
- Contract-first: assert promise → test → code
- Or: lock existing Annex B regexp IdentityEscape that the `v` flag still fails to reject at parse

**Current behavior:**
Without `u`/`v`, incomplete `\u` already matches as IdentityEscape. With `u`, parse already rejects. With `v`, parse still accepts and Node then throws at run.

**Desired behavior:**
Incomplete `\u` (not four hex digits and not `\u{…}`) is IdentityEscape without `u`/`v`. Complete `\uXXXX` stays a UnicodeEscape. Those incomplete forms are early SyntaxError with `u` or `v`.

**Key interfaces:**
- `validate_regexp_literal` / Pattern flag-dependent early errors
- Conformance `.drac` + `.meta` under annex-b, harness `annex_b`

**Acceptance criteria:**
- [x] `/\u/` `.test("u")` is true; `/\u0041/` `.test("A")` is true; incomplete short forms match the IdentityEscape text
- [x] `/\u/u` and `/\u/v` fail compile with an invalid regular expression diagnostic
- [x] `cargo test -p draconic-conformance --test annex_b` prints `test result: ok.`
- [x] E18.44.02 is `done`; E18.44 stays `todo`

**Out of scope:**
- E18.44.01 octal / `\x` remainder under `v` beyond this `\u` child
- Native observations
- Marking E18.44 done
