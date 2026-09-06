---
id: "task-580-e17-02-169-html-comments"
title: "E17.02.169 Annex B.1.3 HTML-like comments without with"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-579-e17-02-169-html-comments"
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T11:49:42Z"
---

# E17.02.169 Annex B.1.3 HTML-like comments without with

## Blocked by

None.

## Done

ROADMAP E17.02.169 is green on the js target: `<!--` single-line open, line-start `-->` single-line close, `ident-->0` is postfix decrement, commented-out statements are not evaluated.

## Context

Roadmap ID **E17.02.169** (child of E17.02). E17.02.122 locked HTML comments only through `with`. E18.08 locked a thin script under the annex-b harness. This sitting files the non-with legacy lock. Leave E17.02 `todo`. Not E17.01, E18.44, N08.15, or Test262 full.

CHECK timeout for O1/O2 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes. If review adds a workspace CHECK, set `ORACLE_CHECK_TIMEOUT_MS=900000`.

## Verify

`cargo test -p draconic-conformance --test legacy` prints `test result: ok.` `cargo test -p draconic-lexer --lib` prints `test result: ok.` E17.02.169 is `done` on ROADMAP.md; E17.02 stays `todo`.

scope: `ROADMAP.md` (E17.02.169), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, `crates/draconic-lexer` skip_trivia as needed

## Links

[[slice-579-e17-02-169-html-comments]] [[ticket-202-e17-02-non-strict-legacy]]

## Agent Brief

**Category:** enhancement
**Summary:** Lock Annex B.1.3 HTML-like comments on js without `with`.

**Intent (required when product behaviour changes):**
- Promise ids: `language.ecma:annex-b`
- Purpose: [[docs/specs/draconic/language/ecma/contract]]
- Contract-first: assert promise → test → code

**Current behavior:**
The legacy harness has no non-with HTML-comment fixture. E17.02.122 covers comments only through `with`. E18.08 covers a thin annex-b script (`<!--` at BOF, mid-line after semicolon, line-start `-->`, `d-->0` postfix decrement). Lexer skip_trivia already tokenizes the core open/close cases.

**Desired behavior:**
In sloppy script, `<!--` is a single-line open comment. Line-start `-->` after newline or whitespace is a single-line close. `ident-->0` is postfix decrement, not a close comment. Commented-out statements are not evaluated. A `"use strict"` script still allows these comments because they are lexical.

**Key interfaces:**
- `skip_trivia` / HTML comment lexing (`Lexer::new` on, `Lexer::new_module` off)
- Conformance `.drac` + `.meta` under es/legacy, harness `legacy` present and runs tests

**Acceptance criteria:**
- [ ] `<!--` and line-start `-->` do not evaluate the commented text
- [ ] `ident-->0` is postfix decrement of `ident`
- [ ] `cargo test -p draconic-conformance --test legacy` prints `test result: ok.`
- [ ] `cargo test -p draconic-lexer --lib` prints `test result: ok.`
- [ ] E17.02.169 is `done`; E17.02 stays `todo`

**Out of scope:**
- E17.02.122 with-plus HTML comments
- E18.08 annex-b harness fixture as the whole bar
- E18.44 regexp IdentityEscape
- Native observations
- Marking E17.02 done
- Workspace CHECK as this task's oracle
