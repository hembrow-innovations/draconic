---
id: "rounds-842-global-object-identifier"
title: "Global object identifier"
kind: round
sitting_kind: wayfinder
status: awaiting-answers
tags: [wayfinder]
created_at: "2026-09-12T05:45:00Z"
updated_at: "2026-09-12T06:48:45Z"
---

# Global object identifier

## Round 1

### Questions

1. Promote [[ticket-841-use-global-not-globalthis]] into work?
2. What should promoting change: alias, teaching-only, or replace `globalThis` everywhere?

### Answers

1. Promote into work.
2. Replace `globalThis` everywhere. That rewrites ROADMAP E15.01, `language.ecma:builtins`, and [[0004-full-ecma-262-and-embed]]. Escalated to this wayfinder sitting.

## Round 2

### Questions

1. After research, does a Draconic Program still see `globalThis` as the ECMA-262 global object identifier?

### Answers

1. No. Programs do not see `globalThis`. The Program-visible identifier is `global`. Reopen [[0004-full-ecma-262-and-embed]] and rewrite E15.01 plus `language.ecma:builtins`.

## Round 3

### Questions

1. Does a Program see free `console`? If so, is it the host print object, the L06 logger, or both?

### Answers

1. Yes. Free `console` is the host print object per target. `createLogger` stays separate.

## Confirm

## Objectives

Decide the Program-visible identifier for the global object: keep ECMA-262 `globalThis`, add a host `global` alias, or replace `globalThis` with `global`.

## Decisions so far

- [[ticket-843-ecma-host-global-names|What do ECMA-262 and hosts name the global object?]]. ECMA-262 is `globalThis`; Node `global` is legacy.
- [[ticket-844-program-visible-globalthis|Does a Program still see globalThis?]]. No. Programs see `global`. Reopen ADR-0004 and rewrite E15.01 plus builtins.
- [[ticket-855-free-console-builtin|Does a Program see free console?]]. Yes. Host print object per target; `createLogger` stays separate.

## Not yet specified

- Browser `window` / `self` host aliases.
- How Test262 and native console lowering move off Program-visible `globalThis` (waits on [[ticket-854-globalthis-property]]).
- What [[0004-full-ecma-262-and-embed]] and [[location-218-conformance]] say after the property decision.

## Out of scope

- Bit-identical Node or V8.
- Writing slices or tasks from this round.
- Marking E17.02 or E18.44 done.
- Node `global` as an extra host alias ([[ticket-845-node-global-alias]] dropped).
- Teaching console bind from the global object ([[ticket-846-teaching-console-bind]] dropped).
