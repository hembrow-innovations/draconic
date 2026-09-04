---
id: "s-e17-02-remainder-workspace-timeout"
title: "E17.02 remainder workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:05:46Z"
updated_at: "2026-09-04T15:54:37Z"
claimed-by: 53011f5e-2715-4cda-8baf-4aa073a90f66
---

# E17.02 remainder workspace tests finish

## Why

Review of [[s-e17-02-remainder]] left ROADMAP E17.02 unfinished: O1 (legacy harness) held, but O2 `cargo test --workspace` timed out at 120s. The conformance location still needs the E17.02 remainder Loop to leave the workspace green, not only the `legacy` crate test.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP E17.02 remainder work. The `legacy` harness stays green. If the hang comes from the E17.02 remainder change, fix that remainder so both checks hold. Leave E17.02 `todo` while untracked remainder remains. Mark E17.02 `done` only when no untracked remainder stays.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-e17-02-remainder]]**: that slice stays sealed `failed`
- **Re-opening [[s-e17-02]]**: that slice stays sealed `failed`
- **E17.01**: `with` statement basics already done
- **E18.44**: untracked ECMA-262 remainder outside this legacy bucket
- **N08.15**: native observations of non-strict legacy
- **Test262 full allowlist**: not this slice

## Oracle checklist

- [x] O1: workspace tests finish after the E17.02 remainder Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test legacy --offline && cargo test -p draconic-conformance --test harness --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=3e3653ade6a6e906 bytes=110651 at=2026-09-04T15:53:58.857Z

- [x] O2: E17.02 remainder fixtures stay green on the declared js target through the legacy harness
  CHECK: cargo test -p draconic-conformance --test legacy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=4c72dfb92ad7d8d0 bytes=16873 at=2026-09-04T15:54:02.970Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[e17-02-remainder-workspace-timeout]]`

## See also

ROADMAP.md E17.02, `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, CONTEXT.md, [[conformance]], [[s-e17-02]], [[s-e17-02-remainder]], [[s-e17-02-workspace-timeout]], [[ticket-137-e17-02-remainder-workspace-timeout]].
