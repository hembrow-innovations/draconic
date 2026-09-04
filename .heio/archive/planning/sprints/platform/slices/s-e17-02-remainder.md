---
id: "s-e17-02-remainder"
title: "E17.02 other non-strict legacy remainder"
kind: slice
status: failed
sprint: "platform"
tags: []
created_at: "2026-09-02T11:04:43Z"
updated_at: "2026-09-04T15:04:08Z"
claimed-by: 502bd083-3feb-49e7-9ff4-3f9294f21635
---

# E17.02 other non-strict legacy remainder

## Why

ROADMAP E17.02 is the leftover non-strict legacy bucket beyond `with` (E17.01). Tracked children stay done; untracked remainder stays here so the conformance location does not pretend the sloppy-mode Annex B / arguments / eval / PutValue gaps are closed. [[s-e17-02]] stayed sealed `failed`; [[s-e17-02-workspace-timeout]] met the leftover workspace check. This sitting continues the E17.02 remainder.

## Done

One atomic untracked remainder of ROADMAP E17.02 is implemented test-first on the js target. Fixtures live under `tests/conformance/fixtures/es/legacy`. The `legacy` harness in `tests/conformance/tests/legacy.rs` is green for that remainder. If E17.02 is still larger than one sitting, split one child under E17.02 and complete only that child this Loop; leave E17.02 `todo` while untracked remainder remains. Mark E17.02 `done` only when no untracked remainder stays.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-e17-02]]**: that slice stays sealed `failed`
- **E17.01**: `with` statement basics already done
- **E18.44**: untracked ECMA-262 remainder outside this legacy bucket
- **N08.15**: native observations of non-strict legacy
- **Test262 full allowlist**: not this slice

## Oracle checklist

- [x] O1: E17.02 remainder fixtures run on the declared js target through the legacy harness
  CHECK: cargo test -p draconic-conformance --test legacy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=930821f14ce00c21 bytes=16873 at=2026-09-04T15:01:44.626Z

- [ ] O2: workspace tests stay green after the E17.02 remainder Loop
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=timeout match=yes bytes=85144 at=2026-09-04T15:03:44.627Z
  ABANDON: leftover after --reverify; CHECK timed out at 120s (exit=timeout match=yes bytes=85144); home [[ticket-137-e17-02-remainder-workspace-timeout]]

## Pool

Durable links to task-pool ids. Never drop them.

- [[e17-02-168-assign-update-target]]

## See also

ROADMAP.md E17.02, `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, CONTEXT.md, [[conformance]], [[s-e17-02]], [[s-e17-02-workspace-timeout]], [[ticket-29-e17-02-non-strict-legacy]].
