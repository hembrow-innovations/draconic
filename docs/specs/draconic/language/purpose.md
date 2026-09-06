---
id: "purpose"
title: "Language purpose"
kind: purpose
description: "Product brief for Draconic Programs: job, scope, non-goals."
status: active
domain: draconic
area: language
tags: [purpose]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Language purpose

## Job

Draconic is the language product: source Programs, their meaning, and the surface syntax developers write. A Program is a full ECMAScript superset with TypeScript-inspired static types and native systems types.

## In scope

- Program meaning for shipped ECMA clusters (expressions, statements, functions, objects, classes, arrays, strings, numbers, collections, proxies, modules, async, generators, eval, built-ins, `with`, Annex B as implemented)
- Checker behaviour that is TypeScript-inspired and not tsc-compatible, including diagnostics on type mismatch
- Dual worlds: JS values and native types in one Program at explicit boundaries; portable vs native-only vs JS-only
- Native types as unboxed systems types, not JavaScript primitives
- Catchable exceptions (`try` / `catch` / `finally` / `throw`) versus process abort

## Out of scope

Hard fences.

- Toolchain product (Compiler, CLI, Frontend, IR, backends as products) — other spec trees
- Host I/O, packages, and the Conformance harness as products — other spec trees
- Roadmap **E17.02** untracked remainder (non-strict legacy beyond filed children)
- tsc compatibility, compiling existing TypeScript projects, or emitting TypeScript
- Claiming untested ECMA-262 clauses as shipped Program meaning
- Inventing product rules not in [[CONTEXT]], [[ROADMAP]] done items, or the ADRs below
- Implementing compiler features or adding Conformance fixtures

## Surfaces

Developers meet this area as source Programs (file or string) and as the Learn and Reference language pages.

## Authority

- Behaviour: [[specs/draconic/language/ecma/contract]], [[specs/draconic/language/types/contract]], [[specs/draconic/language/dual-worlds/contract]], [[specs/draconic/language/native-types/contract]], [[specs/draconic/language/exceptions/contract]]
- Tests: [[specs/draconic/language/ecma/test]], [[specs/draconic/language/types/test]], [[specs/draconic/language/dual-worlds/test]], [[specs/draconic/language/native-types/test]], [[specs/draconic/language/exceptions/test]]
- Decisions: [[0004-full-ecma-262-and-embed]], [[0005-ts-inspired-not-tsc]], [[0003-gc-runtime-and-dual-worlds]], [[0011-catchable-exceptions-vs-abort]], [[0007-test262-staged-roll-in]]
- Glossary: [[CONTEXT]]
- Completeness tracker: [[ROADMAP]]

## Open product questions

- (none)
