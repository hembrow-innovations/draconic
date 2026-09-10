---
id: "task-789-annex-b-honesty"
title: "Park annex-b native until the walker exists"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "dragons-audit"
slice: "slice-782-annex-b-native-honesty"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-10T05:30:00Z"
---

# Park annex-b native until the walker exists

## Blocked by

None.

## Done

annex_b tests are green. async_methods, private_methods, and static_private_fields are js-only. N08.16.35, N08.16.37, N08.16.38 are not `done`.

## Context

Those three fixtures declare `targets: js,native` and fail native emit with unsupported IR. ROADMAP marks the native observation rows done. Honesty: drop native from those three `.meta` files and set the three ROADMAP rows to `todo`. Do not add a fingerprint adapter to make them green. Restoring native is [[task-794-annex-b-native-through-walker]].

## Verify

Slice O1 and O2.

scope: tests/conformance/fixtures/es/annex-b/async_methods.meta, private_methods.meta, static_private_fields.meta, ROADMAP.md those three rows only

## Links

[[slice-782-annex-b-native-honesty]] [[ticket-775-annex-b-native-false-green]] [[task-794-annex-b-native-through-walker]]

## Agent Brief

**Category:** honesty
**Summary:** Un-done false-green native annex-b rows. Do not implement lowering.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: N08.16.35 N08.16.37 N08.16.38
- Purpose: completeness matches the suite
- Contract-first: do not invent native lowering here

**Current behavior:**
Three `_runs` tests fail native unsupported IR. ROADMAP says done.

**Desired behavior:**
annex_b suite green. Those rows not done. Fixtures js-only.

**Key interfaces:**
- fixture `.meta` targets
- ROADMAP.md status cells

**Acceptance criteria:**
- [ ] slice O1 annex_b ok
- [ ] slice O2 honest
- [ ] no new walk_* adapter
- [ ] no LLVM emit changes

**Out of scope:**
- implementing native lowering
- other annex-b fixtures
- E18 parent remainder

**Explain this part:**
A new fingerprint adapter is a regression. Park native. The walker slice restores it.
