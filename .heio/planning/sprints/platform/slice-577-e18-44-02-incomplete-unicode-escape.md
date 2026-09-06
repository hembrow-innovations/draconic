---
id: "slice-577-e18-44-02-incomplete-unicode-escape"
title: "E18.44.02 Annex B.1.4 incomplete UnicodeEscape IdentityEscape"
kind: slice
status: met
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-06T11:31:00Z"
updated_at: "2026-09-06T11:40:35Z"
---

# E18.44.02 Annex B.1.4 incomplete UnicodeEscape IdentityEscape

## Why

ROADMAP E18.44 stays the leftover ECMA-262 bucket. This cut files one discovered child: Annex B.1.4 incomplete `\u` IdentityEscape without `u`/`v`, and early SyntaxError with those flags.

## Done

ROADMAP E18.44.02 is implemented test-first on the js target. Fixtures under `tests/conformance` `es/annex-b` plus the `annex_b` harness are green. E18.44 stays `todo`.

## Blocked by

None.

## Non-goals

- **E01–E18.43 / E18.44.01**: already tracked
- **E17.02**: other non-strict legacy remainder
- **S02 / E19.02**: Test262 allowlist expansion
- **N08.16**: native observations of annex-b fixtures
- Marking E18.44 done
- Dropping E18.44 without filing finer rows

## Oracle checklist

- [x] O1: E18.44.02 fixtures run on the declared js target through the annex-b harness
  CHECK: cargo test -p draconic-conformance --test annex_b
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes 104 passed wall=15.01s at=2026-09-06T11:40:35Z

- [x] O2: lexer regexp unit tests stay green for incomplete `\u` IdentityEscape
  CHECK: cargo test -p draconic-lexer --lib
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes 58 passed at=2026-09-06T11:40:35Z

## Pool

Durable links to task ids. Never drop them.

- `[[task-578-e18-44-02-incomplete-unicode-escape]]`

## See also

ROADMAP.md E18.44 / E18.44.02, `tests/conformance/fixtures/es/annex-b`, `tests/conformance/tests/annex_b.rs`, CONTEXT.md, [[location-218-conformance]], [[ticket-203-e18-44-untracked-ecma-262-remainder-beyond]].
