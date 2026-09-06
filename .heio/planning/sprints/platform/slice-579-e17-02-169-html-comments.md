---
id: "slice-579-e17-02-169-html-comments"
title: "E17.02.169 Annex B.1.3 HTML-like comments without with"
kind: slice
status: frozen
sprint: "platform"
blocked_by: []
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T11:49:42Z"
---

# E17.02.169 Annex B.1.3 HTML-like comments without with

## Why

ROADMAP E17.02 stays the leftover non-strict legacy bucket. This cut files one discovered child: Annex B.1.3 HTML-like comments in sloppy script without wrapping the program in `with`.

## Done

ROADMAP E17.02.169 is implemented test-first on the js target. Fixtures under `tests/conformance` `es/legacy` plus the `legacy` harness are green. E17.02 stays `todo`.

## Blocked by

None.

## Non-goals

- **E17.01 / E17.02.122**: `with` basics and `with` + HTML comments
- **E18.08 / E18.44**: annex-b harness HTML comments and regexp IdentityEscape
- **S02 / E19.02**: Test262 allowlist expansion
- **N08.15**: native observations of non-strict legacy
- Marking E17.02 done
- `cargo test --workspace` as this slice's oracle (workspace budget is [[slice-216-workspace-test-budget]] / ADR-0012 ten minutes)

## Oracle checklist

- [ ] O1: E17.02.169 fixtures run on the declared js target through the legacy harness
  CHECK: cargo test -p draconic-conformance --test legacy
  EXPECT: test result: ok.
  EVIDENCE: pending

- [ ] O2: lexer trivia unit tests stay green for HTML-like comments
  CHECK: cargo test -p draconic-lexer --lib
  EXPECT: test result: ok.
  EVIDENCE: pending

## Pool

Durable links to task ids. Never drop them.

- `[[task-580-e17-02-169-html-comments]]`

## See also

ROADMAP.md E17.02 / E17.02.169, `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, CONTEXT.md, [[location-218-conformance]], [[ticket-202-e17-02-non-strict-legacy]].
