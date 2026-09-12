---
id: "task-901-adr-0004-draft-wording"
title: "Draft proposed ADR-0004 and location-218 sentences"
kind: task
status: ready
mode: hitl
blocked_by:
  - task-897-browser-window-self
  - task-899-test262-absent-globalthis
sprint: "global-object-followup"
slice: "slice-900-adr-0004-global-wording"
tags: [wayfinder]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Draft proposed ADR-0004 and location-218 sentences

## Blocked by

[[task-897-browser-window-self]]: window and self answers first.

[[task-899-test262-absent-globalthis]]: Test262 honesty mechanism first.

## Done

Draft proposed ADR and location sentences recorded for a later map sitting. Do not patch location-218 or ADR-0004.

## Context

Escalate-shaped if applied in a workflow. This sitting drafts only. location-218 destination remains leftover ECMA-262 rows and Test262 are honest on both required targets until a map sitting says otherwise.

## Verify

Drafts live on ticket-878 Notes. location-218 destination sentence unchanged. ADR-0004 unchanged. Drain must not claim.

scope: `.heio/planning/tickets/ticket-878-adr-0004-global-wording.md`. Forbidden: ADR-0004 file, location-218 file, ROADMAP, compiler, Test262 harness, website

## Links

[[slice-900-adr-0004-global-wording]] [[ticket-878-adr-0004-global-wording]] [[0004-full-ecma-262-and-embed]] [[location-218-conformance]]

## Agent Brief

**Category:** planning
**Summary:** After 876 and 877 answers exist, draft proposed ADR-0004 and location-218 sentences for a later map sitting. Do not apply them.

**Drain:** HITL. Blocked by the window-or-self answer and the Test262-honesty answer. Do not claim. Do not patch vault files.

**Skills:** management, docs, domain-modeling. Stop if the work would edit location-218 or ADR-0004.

**Vault pack:** [[rounds-842-global-object-identifier]], [[0004-full-ecma-262-and-embed]], [[location-218-conformance]], [[ticket-844-program-visible-globalthis]], [[ticket-854-globalthis-property]], answers from 876 and 877.

**Intent:** **No product behaviour change**. Workflow must not rewrite location destination sentences.

**Current behavior:**
ADR-0004 says the destination is literally all of ECMA-262, including eval, new Function, and with. location-218 destination is leftover ECMA-262 rows and Test262 are honest on both required targets. Programs will see global, with no globalThis identifier or property. Exact replacement sentences are unwritten.

**Desired behavior:**
Once 876 and 877 have answers, draft proposed sentences on this ticket. ADR-0004 almost certainly needs a proposal because literally-all is in tension with dropping globalThis. location-218 may keep its current destination if Test262 honesty still holds; only propose a new location-218 destination if those answers require it. Record drafts for a later map sitting. Do not edit ADR-0004, location-218, the heio roadmap location bullet, ROADMAP E15.01, or `language.ecma:builtins`.

**Key interfaces:**
- ADR-0004 destination sentence
- location-218 This is working when sentence, currently leftover ECMA-262 rows and Test262 are honest on both required targets

**Acceptance criteria:**
- [ ] 876 and 877 answers exist first
- [ ] Proposed ADR-0004 sentence recorded on this ticket
- [ ] location-218 either keep-as-is or a proposed sentence, recorded not applied
- [ ] ADR-0004 file and location-218 file unchanged

**Out of scope:**
- Patching those files
- Implementing global, window, self, or Test262 rewrite
- Website chrome
- Replacing ROADMAP.md with heio
