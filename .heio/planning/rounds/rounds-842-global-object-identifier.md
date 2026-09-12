---
id: "rounds-842-global-object-identifier"
title: "Global object identifier"
kind: round
sitting_kind: wayfinder
status: awaiting-confirm
tags: [wayfinder]
created_at: "2026-09-12T05:45:00Z"
updated_at: "2026-09-12T18:40:00Z"
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

## Round 4

### Questions

1. Does the global object still expose a `globalThis` property (`global.globalThis`) for ECMA-262 interop?

### Answers

1. No. No identifier and no property. Reopen [[0004-full-ecma-262-and-embed]] and rewrite E15.01 plus `language.ecma:builtins`. Test262 honesty treats `globalThis` as absent.

## Round 5

### Questions
1. Does a Program see window or self as names for the global object?
2. How does Test262 treat absent globalThis?

### architect

#### Question 1 - window or self
Does a Program see window or self as names for the global object?

#### Answer
window: no; self: no

#### Reasoning
The sitting already locked one Program-visible identifier: `global`. `window` and `self` would be extra host aliases beyond that language name, the same class as the dropped Node extra alias. Vault text does not require them. ADR-0004 is full ECMA-262 including eval, `new Function`, and `with`, not a browser engine. Intent will not ship a full browser engine or bit-identical Node or V8. Teaching todo binds `document` and `localStorage` from the global object, not `window`. Checker builtin install has no `window` or `self` today.

#### Question 2 - Test262 absent globalThis
How does Test262 treat absent globalThis?

#### Answer
honest-fail-and-allowlist

#### Reasoning
Round 4 already named honesty: Test262 treats `globalThis` as absent. That frame is honesty, not rewrite. ADR-0007 keeps official Test262 as the external bar; failures stay on the allowlist and baseline rather than rewriting the suite to look like Draconic. A harness rewrite of `globalThis` to `global` would hide the language divergence and make those tests a false green.

### Answers
1. window: no; self: no
2. honest-fail-and-allowlist

## Confirm

## Objectives

Decide the Program-visible identifier for the global object: keep ECMA-262 `globalThis`, add a host `global` alias, or replace `globalThis` with `global`.

## Decisions so far

- [[ticket-843-ecma-host-global-names|What do ECMA-262 and hosts name the global object?]]. ECMA-262 is `globalThis`; Node `global` is legacy.
- [[ticket-844-program-visible-globalthis|Does a Program still see globalThis?]]. No. Programs see `global`. Reopen ADR-0004 and rewrite E15.01 plus builtins.
- [[ticket-855-free-console-builtin|Does a Program see free console?]]. Yes. Host print object per target; `createLogger` stays separate.
- [[ticket-854-globalthis-property|Does the global object still expose globalThis?]]. No. No identifier and no property.
- [[ticket-876-browser-window-self|Does a Program see window or self?]]. No. Neither `window` nor `self`.
- [[ticket-877-test262-absent-globalthis|How does Test262 treat absent globalThis?]]. Honest-fail-and-allowlist.

## Not yet specified

[[ticket-878-adr-0004-global-wording]] waits on these recorded answers.

## Out of scope

- Bit-identical Node or V8.
- Writing slices or tasks from this round.
- Marking E17.02 or E18.44 done.
- Node `global` as an extra host alias ([[ticket-845-node-global-alias]] dropped).
- Teaching console bind from the global object ([[ticket-846-teaching-console-bind]] dropped).
- Inbound teaching bind via `global` instead of `globalThis` ([[ticket-841-use-global-not-globalthis]] dropped).
- How native `globalThis.console.log` lowering moves off `globalThis` (later planning sitting; free `console` is decided).
