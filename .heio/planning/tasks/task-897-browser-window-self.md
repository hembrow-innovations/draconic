---
id: "task-897-browser-window-self"
title: "Record whether Programs see window or self"
kind: task
status: ready
mode: hitl
blocked_by: []
sprint: "global-object-followup"
slice: "slice-896-browser-window-self"
tags: [wayfinder]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Record whether Programs see window or self

## Blocked by

None.

## Done

Answer recorded: window yes or no; self yes or no.

## Context

Programs will see `global`, not `globalThis`. Node `global` is not an extra alias. Browser research says hosts historically expose `window`, `self`, and `frames`. Intent non-goal: bit-identical Node or V8. This sitting records the answer. It does not implement aliases.

## Verify

Ticket Notes contain `Answer:` with both names. Drain must not claim. No language change.

scope: `.heio/planning/tickets/ticket-876-browser-window-self.md`, `.heio/planning/rounds/rounds-842-global-object-identifier.md`

## Links

[[slice-896-browser-window-self]] [[ticket-876-browser-window-self]] [[rounds-842-global-object-identifier]]

## Agent Brief

**Category:** planning
**Summary:** Record whether a Program sees browser window and/or self as names for the global object.

**Drain:** HITL. Do not claim. Do not implement.

**Skills:** management, docs, domain-modeling. Do not load rust-development to change the language.

**Vault pack:** [[rounds-842-global-object-identifier]], [[ticket-843-ecma-host-global-names]], [[ticket-845-node-global-alias]], [[ticket-844-program-visible-globalthis]], intent We will not, [[location-223-stdlib]] small honest surfaces.

**Intent:** **No product behaviour change**. Do not edit `language.ecma:builtins`.

**Current behavior:**
Programs still see Checker builtin globalThis. Wayfinder already decided the Program-visible identifier will be global, not globalThis, and Node global is not an extra alias. Browser research says hosts historically expose window, self, and frames; workers have self. Teaching todo binds document and localStorage from the global object, not window. Intent non-goal: bit-identical Node or V8.

**Desired behavior:**
A human records whether Programs also see window and/or self as names for the global object. Either name may be yes or no independently. frames only if the human names it. No compiler, ROADMAP, or vault patch in this sitting.

**Key interfaces:**
- Checker builtin install, currently globalThis only. Host aliases are extra names beyond the language identifier.

**Acceptance criteria:**
- [ ] Answer records window yes or no
- [ ] Answer records self yes or no
- [ ] No host-alias implementation
- [ ] Test262 policy and ADR wording left to 877 and 878

**Out of scope:**
- Test262 allowlists
- ADR-0004 and location-218 sentences
- Implementing aliases
- Native globalThis.console.log lowering
- Website chrome
