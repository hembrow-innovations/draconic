---
id: "task-899-test262-absent-globalthis"
title: "Record how Test262 treats absent globalThis"
kind: task
status: ready
mode: hitl
blocked_by: []
sprint: "global-object-followup"
slice: "slice-898-test262-absent-globalthis"
tags: [wayfinder]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Record how Test262 treats absent globalThis

## Blocked by

None.

## Done

Answer recorded: honest-fail-and-allowlist, or harness-rewrite.

## Context

`globalThis` is gone as identifier and property. Test262 is the external ECMA-262 bar. The harness still uses `globalThis`. This sitting records one mechanism. It does not patch harness, allowlist, or [[location-218-conformance]].

## Verify

Ticket Notes contain `Answer:` with exactly one of honest-fail-and-allowlist or harness-rewrite. Drain must not claim. No harness or allowlist patch.

scope: `.heio/planning/tickets/ticket-877-test262-absent-globalthis.md`, `.heio/planning/rounds/rounds-842-global-object-identifier.md`

## Links

[[slice-898-test262-absent-globalthis]] [[ticket-877-test262-absent-globalthis]] [[location-218-conformance]]

## Agent Brief

**Category:** planning
**Summary:** Record whether Test262 tests that use globalThis fail honestly on the allowlist, or the harness rewrites globalThis to global.

**Drain:** HITL. Do not claim. Do not implement.

**Skills:** management, docs, domain-modeling. Do not change the Test262 harness.

**Vault pack:** [[rounds-842-global-object-identifier]], [[ticket-854-globalthis-property]], [[0007-test262-staged-roll-in]], [[location-218-conformance]], `language.ecma:test262-staged-allowlist`, [[ticket-855-free-console-builtin]].

**Intent:** **No product behaviour change**. Do not edit allowlist or harness.

**Current behavior:**
Harness and shims use globalThis. fnGlobalObject returns globalThis. Round 4 and ticket 854 say Test262 honesty treats globalThis as absent. That is the frame, not the mechanism. Free host console is decided. Native globalThis.console.log lowering already shipped; moving that lowering is a later sitting.

**Desired behavior:**
A human records one mechanism: tests that use globalThis fail honestly and stay on the allowlist, or the harness rewrites globalThis to global. Do not mix. Do not expand Test262 policy beyond that binary. Do not patch harness, allowlist, ADR-0004, or location-218.

**Key interfaces:**
- Test262 staged allowlist plus harness. location-218 destination stays: leftover ECMA-262 rows and Test262 are honest on both required targets.

**Acceptance criteria:**
- [ ] Answer is exactly one of honest-fail-and-allowlist or harness-rewrite
- [ ] No harness or allowlist edit
- [ ] location-218 destination sentence unchanged
- [ ] ADR wording left to 878

**Out of scope:**
- Writing ADR-0004 or location-218 sentences
- Implementing rewrite
- Native console lowering move
- Website chrome
