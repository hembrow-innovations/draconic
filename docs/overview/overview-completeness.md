---
id: overview-completeness
title: Language completeness
kind: overview
domain: draconic
area: overview
tags: [overview]
created_at: "2026-09-06"
updated_at: "2026-09-06"
description: How language completeness is defined by the archived Roadmap, the Conformance suite, Loop tracks, and locked ADRs.
---

# Language completeness

## Overview

[[ROADMAP]] is the archived Loop checklist together with the Conformance suite. A Roadmap item is `done` only when its tests are green on every applicable target (`js`, `native`, both, or `compiler`). This note does not copy the checklist. Glossary terms Loop, Roadmap, and Conformance suite live in [[CONTEXT]]. Layout is [[domain]]. Hub siblings are [[overview-toolchain]] and [[overview-vault]].

## Context

Language completeness is built by one mega-loop ([[0006-mega-loop-roadmap-tests]]): pick the next incomplete Roadmap item, implement it test-first, mark it done only when tests pass. A pure LLM-judged queue and a ticket-less void were rejected as too drift-prone.

`.heio/` occupancy is not completeness. Do not replace [[ROADMAP]] with Heio tickets or slices. Prefer `cargo test --workspace` and the `draconic` CLI. Empty board means stop: if there are zero `todo` rows, do not invent work.

## Design

### Done bar

- **Source of truth**: [[ROADMAP]] plus tests.
- **Status values**: `todo`, `in_progress`, `done`, `blocked`.
- **Green means**: the item’s listed tests pass on applicable targets. Native / both rows assert program results on native, not a hello-stub fallback.
- **Conformance suite**: pins ECMA-262 and native-type behavior (`tests/conformance`). Test262 is a staged external bar ([[0007-test262-staged-roll-in]]), not day-one full suite.
- **Loop update rules**: claim one item; split clusters rather than marking them done without coverage; never delete ECMA-262 obligations.

### Spine versus platform versus product

Historical **B / E / T / N / U** remain the language and toolchain spine. Prefer draining open spine `todo`s when Loop intent is conformance. Platform and product tracks are still real Loop work when the intent is building programs.

- **B Bootstrap**: lexer through first JS and native hello, CLI build.
- **E ECMA-262 Conformance**: grow via Loop; clusters split into child rows; includes staged Test262 under E19.
- **T Types**: Checker; TypeScript-inspired ([[0005-ts-inspired-not-tsc]], [[architecture-check]]).
- **N Native types and LLVM**: unboxed types, Runtime, Dual-world lowering ([[architecture-dual-worlds]], [[architecture-backend-llvm]]).
- **U Tooling**: CLI test runner, diagnostics, fmt, LSP, REPL, and related compiler UX ([[architecture-cli]], [[architecture-lsp]], [[architecture-diagnostics]]). The Roadmap heading is Tooling; item IDs are `U…`.

Platform tracks (capability-class host and ship work; not identical APIs to Rust, Go, or Node):

- **H Host I/O and networking**: process, stdio, fs, sockets-first then thin HTTP ([[0008-host-io-sockets-first-http]]).
- **K Packages**: Go-style git-backed modules ([[0009-go-style-git-packages]], [[architecture-pkg]]).
- **F FFI and systems interop**.
- **C Concurrency and parallelism**.
- **L Stdlib libraries**.
- **D Distribution and install**.
- **R Reliability and security**: catchable exceptions versus abort, permissions, sandbox ([[0011-catchable-exceptions-vs-abort]], [[security]], [[reliability]]).

Product and external spec tracks:

- **P Product**: examples, README honesty, docs site skeleton. Native depth stays under N; host under H; packages under K.
- **S Spec external**: Test262 staged; point at E19 rather than duplicating harness work.

Platform critical paths on the Roadmap (do not expand here): H toward http-echo; K toward temp-git e2e; F toward first C call; C toward worker plus channel; D toward install smoke; L and R claim v1-bar todos.

### Locked ADRs

Do not invent decisions. Wikilink existing files in `docs/adr/`:

- **0001**: [[0001-rust-host-compiler]] — Rust host Compiler; no self-host.
- **0002**: [[0002-shared-ir-dual-backends]] — shared IR; JS and LLVM backends.
- **0003**: [[0003-gc-runtime-and-dual-worlds]] — tracing GC Runtime and Dual worlds.
- **0004**: [[0004-full-ecma-262-and-embed]] — full ECMA-262 and Embed for eval.
- **0005**: [[0005-ts-inspired-not-tsc]] — TypeScript-inspired Checker, not tsc.
- **0006**: [[0006-mega-loop-roadmap-tests]] — mega-loop driven by Roadmap and tests.
- **0007**: [[0007-test262-staged-roll-in]] — Test262 staged roll-in.
- **0008**: [[0008-host-io-sockets-first-http]] — sockets-first host I/O, then thin HTTP.
- **0009**: [[0009-go-style-git-packages]] — git-backed packages.
- **0010**: [[0010-public-docs-draconic-ssg]] — public docs from `website/`, not this vault.
- **0011**: [[0011-catchable-exceptions-vs-abort]] — catchable exceptions versus process abort.
- **0012**: [[0012-oracle-check-timeout]] — workspace CHECK timeout is ten minutes, not a hang detector.
- **0013**: [[0013-public-site-tanstack-start]] — public site presentation is TanStack Start.

## Trade-offs

[[0006-mega-loop-roadmap-tests]] keeps completeness on [[ROADMAP]] plus green tests rather than a judged queue. [[0007-test262-staged-roll-in]] keeps Test262 staged on js first. Oracle budget is [[0012-oracle-check-timeout]]: keep `cargo test --workspace` as the workspace oracle when that is the promise.

## Related notes

Toolchain map: [[overview-toolchain]]. Vault map: [[overview-vault]]. Purpose: [[purpose]]. Standard: [[standards-docs-vault]]. Guides: [[guides-toolchain]], [[guides-public-docs]]. APIs: [[api-cli]], [[api-embed]]. Non-functionals: [[security]], [[performance]], [[reliability]].

- **architecture-pipeline**: [[architecture-pipeline]]
- **system-design-compile-pipeline**: [[system-design-compile-pipeline]]
- **architecture-frontend**: [[architecture-frontend]]
- **architecture-lexer**: [[architecture-lexer]]
- **architecture-parser**: [[architecture-parser]]
- **architecture-ast**: [[architecture-ast]]
- **architecture-check**: [[architecture-check]]
- **architecture-diagnostics**: [[architecture-diagnostics]]
- **architecture-ir**: [[architecture-ir]]
- **architecture-backend-js**: [[architecture-backend-js]]
- **architecture-backend-llvm**: [[architecture-backend-llvm]]
- **system-design-dual-backends**: [[system-design-dual-backends]]
- **architecture-runtime**: [[architecture-runtime]]
- **architecture-embed**: [[architecture-embed]]
- **architecture-dual-worlds**: [[architecture-dual-worlds]]
- **system-design-gc-runtime**: [[system-design-gc-runtime]]
- **architecture-cli**: [[architecture-cli]]
- **architecture-lsp**: [[architecture-lsp]]
- **architecture-pkg**: [[architecture-pkg]]
- **architecture-linker**: [[architecture-linker]]
- **architecture-editors**: [[architecture-editors]]
