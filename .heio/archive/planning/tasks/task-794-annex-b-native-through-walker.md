---
id: "task-794-annex-b-native-through-walker"
title: "Restore annex-b native through the walker"
kind: task
status: completed
mode: afk
blocked_by: ["task-793-delete-try-folded-walks", "task-789-annex-b-honesty"]
sprint: "dragons-audit"
slice: "slice-784-llvm-no-fingerprint"
tags: []
created_at: "2026-09-10T05:30:00Z"
updated_at: "2026-09-11T23:59:00Z"
---

# Restore annex-b native through the walker

## Blocked by

[[task-793-delete-try-folded-walks]]: walker exists.
[[task-789-annex-b-honesty]]: fixtures are parked js-only until this sitting.

## Done

`es/annex-b/async_methods`, `private_methods`, and `static_private_fields` declare js and native again and pass native.stdout. N08.16.35, N08.16.37, N08.16.38 are `done` only after those tests are green. Slice O2 holds. No new walk_* fingerprint.

## Context

These programs failed because no adapter claimed them. After the walker exists, lower them for real observations. Do not revive OBS printers. Do not add is_es_async_methods_module.

## Verify

Slice O2. ROADMAP those three rows `done` only with green tests.

scope: crates/draconic-backend-llvm walker, those three fixtures `.meta`, ROADMAP.md those three rows

## Links

[[slice-784-llvm-no-fingerprint]] [[slice-782-annex-b-native-honesty]] [[ticket-775-annex-b-native-false-green]]

## Agent Brief

**Category:** native
**Summary:** Real native observations for three annex-b fixtures through the walker.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: N08.16.35 N08.16.37 N08.16.38
- Purpose: native observations, not B08 hello
- Contract-first: do not invent extra Annex B surface

**Current behavior:**
Honesty parked these js-only.

**Desired behavior:**
targets js,native. native.stdout as in the parked meta. Tests green. ROADMAP done.

**Key interfaces:**
- emit_llvm_ir
- fixture meta
- size-file-budget: no new file over 1000

**Acceptance criteria:**
- [x] slice O2 annex_b ok
- [x] those three ROADMAP rows done
- [x] no walk_* fingerprint
- [x] no const OBS

## Gauntlet

- **round 1**: `cargo test -p draconic-conformance --test annex_b --offline` — win. `test result: ok.` N08.16.35/37/38 native.stdout green. No new `walk_*`. No `is_es_async_methods_module`. No `const OBS`.

**Out of scope:**
- other annex-b remainder
- console.log ([[ticket-773-native-console-log]])
- inkwell

**Explain this part:**
If the walker cannot lower one of the three in this sitting, leave it js-only and ticket. Do not mark ROADMAP done.
