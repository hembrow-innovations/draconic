---
id: "s-e18-44-workspace-timeout"
title: "E18.44 remainder workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:10:20Z"
updated_at: "2026-09-04T15:57:08Z"
claimed-by: fcb2739b-1419-4672-aab4-f67cf9f79459
---

# E18.44 remainder workspace tests finish

## Why

Review of [[s-e18-44]] left ROADMAP E18.44 unfinished: O1 (annex-b harness) held, but O2 `cargo test --workspace` timed out at 120s. The conformance location still needs the E18.44 remainder Loop to leave the workspace green, not only the `annex_b` crate test.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP E18.44 remainder work. The `annex_b` harness stays green. If the hang comes from the E18.44 remainder change, fix that remainder so both checks hold. Leave E18.44 `todo` while untracked remainder remains. Mark E18.44 `done` only when no untracked remainder stays.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-e18-44]]**: that slice stays sealed `failed`
- **E01–E18.43**: tracked children already done
- **E17.02**: other non-strict legacy remainder
- **S02 / E19.02**: Test262 allowlist expansion
- **N08.16**: native observations of annex-b fixtures
- Dropping E18.44 without filing finer rows

## Oracle checklist

- [x] O1: workspace tests finish after the E18.44 remainder Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test annex_b --offline && cargo test -p draconic-conformance --test harness --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=345dfad5d8e4e03a bytes=100345 at=2026-09-04T15:56:40.865Z

- [x] O2: E18.44 remainder fixtures stay green on the declared js target through the annex-b harness
  CHECK: cargo test -p draconic-conformance --test annex_b
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=0cb126c1b1d7e294 bytes=6566 at=2026-09-04T15:56:46.581Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[e18-44-workspace-timeout]]`

## See also

ROADMAP.md E18.44, `tests/conformance/fixtures/es/annex-b`, `tests/conformance/tests/annex_b.rs`, CONTEXT.md, [[conformance]], [[s-e18-44]], [[ticket-138-e18-44-workspace-timeout]].
