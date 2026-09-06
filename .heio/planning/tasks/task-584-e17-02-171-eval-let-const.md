---
id: "task-584-e17-02-171-eval-let-const"
title: "E17.02.171 direct eval of let/const without caller inject"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "platform"
slice: "slice-583-e17-02-171-eval-let-const"
tags: []
created_at: "2026-09-06T11:49:42Z"
updated_at: "2026-09-06T11:49:42Z"
---

# E17.02.171 direct eval of let/const without caller inject

## Blocked by

None.

## Done

ROADMAP E17.02.171 is green on the js target: `eval("let …")` / `eval("const …")` do not inject into the caller; TDZ and redeclaration SyntaxError hold; `eval("var …")` still injects.

## Context

Roadmap ID **E17.02.171** (child of E17.02). E17.02.11 locked sloppy `eval("var")` / `eval("function")` injection. This sitting files lexical eval instantiation. Leave E17.02 `todo`. Not E17.01, E18.44, N08.15, or Test262 full.

CHECK timeout for O1 is the crate-level default. Do not add `cargo test --workspace` at 120s. Workspace completeness already met on [[slice-216-workspace-test-budget]] under ADR-0012 ten minutes. If review adds a workspace CHECK, set `ORACLE_CHECK_TIMEOUT_MS=900000`.

## Verify

`cargo test -p draconic-conformance --test legacy` prints `test result: ok.` E17.02.171 is `done` on ROADMAP.md; E17.02 stays `todo`.

scope: `ROADMAP.md` (E17.02.171), `tests/conformance/fixtures/es/legacy`, `tests/conformance/tests/legacy.rs`, js eval emit as needed

## Links

[[slice-583-e17-02-171-eval-let-const]] [[ticket-202-e17-02-non-strict-legacy]]

## Agent Brief

**Category:** enhancement
**Summary:** Lock direct eval of `let`/`const` so it does not inject into the caller VariableEnvironment.

**Intent (required when product behaviour changes):**
- Promise ids: `language.ecma:eval`
- Purpose: [[docs/specs/draconic/language/ecma/contract]]
- Contract-first: assert promise → test → code

**Current behavior:**
E17.02.11 only locks sloppy `eval("var …")` / `eval("function …")` injection. The `eval_var_inject` fixture never runs `eval("let …")` or `eval("const …")`. JS emit already keeps a direct-eval callee as Identifier `eval`.

**Desired behavior:**
After `eval("let x=1")` / `eval("const x=1")`, caller `typeof x` is `"undefined"`. Inside the eval source the binding is visible after init. Access before init is a TDZ ReferenceError. `eval("let x")` when the caller already has `var x` or `let x` is a SyntaxError. `"use strict"` eval source still does not inject. Indirect `(0, eval)("let x=1")` does not create a `globalThis` property. `eval("var x")` still injects so E17.02.11 does not regress.

**Key interfaces:**
- PerformEval / EvalDeclarationInstantiation for lexical bindings
- Conformance `.drac` + `.meta` under es/legacy, harness `legacy` present and runs tests

**Acceptance criteria:**
- [ ] `eval("let x=1")` does not bind `x` on the caller; `typeof x` is `"undefined"`
- [ ] `eval("let x=1; x")` is `1`; access before init is ReferenceError
- [ ] `eval("var x")` still injects
- [ ] `cargo test -p draconic-conformance --test legacy` prints `test result: ok.`
- [ ] E17.02.171 is `done`; E17.02 stays `todo`

**Out of scope:**
- E16.01–.03 eval basics
- Annex B if-function / html / octal inside eval
- eval in modules or default params
- Native observations
- Marking E17.02 done
- Workspace CHECK as this task's oracle
