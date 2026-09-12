---
id: "rounds-842-global-object-identifier"
title: "Global object identifier"
kind: round
sitting_kind: wayfinder
status: awaiting-answers
tags: [wayfinder]
created_at: "2026-09-12T05:45:00Z"
updated_at: "2026-09-12T05:50:00Z"
---

# Global object identifier

## Round 1

### Questions

1. Promote [[ticket-841-use-global-not-globalthis]] into work?
2. What should promoting change: alias, teaching-only, or replace `globalThis` everywhere?

### Answers

1. Promote into work.
2. Replace `globalThis` everywhere. That rewrites ROADMAP E15.01, `language.ecma:builtins`, and [[0004-full-ecma-262-and-embed]]. Escalated to this wayfinder sitting.

## Confirm

## Objectives

Decide the Program-visible identifier for the global object: keep ECMA-262 `globalThis`, add a host `global` alias, or replace `globalThis` with `global`.

## Decisions so far

- [[ticket-843-ecma-host-global-names|What do ECMA-262 and hosts name the global object?]]. ECMA-262 is `globalThis`; Node `global` is legacy.

## Not yet specified

- Whether free identifier `console` becomes a builtin.
- Browser `window` / `self` host aliases.
- How Test262 and native console lowering would move if the identifier changes.

## Out of scope

- Bit-identical Node or V8.
- Writing slices or tasks from this round.
- Marking E17.02 or E18.44 done.
