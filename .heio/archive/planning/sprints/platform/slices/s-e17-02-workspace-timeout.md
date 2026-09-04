---
id: "s-e17-02-workspace-timeout"
title: "E17.02 remainder workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-02T09:18:00Z"
updated_at: "2026-09-02T21:10:18Z"
---

# E17.02 remainder workspace tests finish

## Why

Review of [[s-e17-02]] left ROADMAP E17.02 remainder unfinished: O1 (legacy harness) held, but O2 `cargo test --workspace` timed out at 120s. The conformance location still needs the E17.02 remainder Loop to leave the workspace green, not only the `legacy` crate test.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP E17.02 remainder work. The `legacy` harness stays green. If the hang comes from the E17.02 remainder change, fix that remainder so both checks hold. Leave E17.02 `todo` while untracked remainder remains. Mark E17.02 `done` only when no untracked remainder stays.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-e17-02]]**: that slice stays sealed `failed`
- **E17.01**: `with` statement basics already done
- **E18.44**: untracked ECMA-262 remainder outside this legacy bucket
- **N08.15**: native observations of non-strict legacy
- **Test262 full allowlist**: not this slice

## Oracle checklist

- [x] O1: workspace tests finish after the E17.02 remainder Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test legacy --offline && cargo test -p draconic-conformance --test harness --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=8b96500122bb13e5 bytes=104506 at=2026-09-02T10:03:24.663Z

- [x] O2: E17.02 remainder fixtures stay green on the declared js target through the legacy harness
  CHECK: cargo test -p draconic-conformance --test legacy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=05e57f3e2d5ebc94 bytes=17557 at=2026-09-02T10:03:28.612Z

## Pool

Durable links to task-pool ids. Never drop them.

- [[e17-02-workspace-timeout]]

## See also

ROADMAP.md E17.02, `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, CONTEXT.md, [[conformance]], [[s-e17-02]], [[ticket-28-e17-02-workspace-timeout]].
