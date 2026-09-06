---
id: "purpose"
title: "Toolchain purpose"
kind: purpose
description: "Product brief: job, scope, and fences for the Draconic Toolchain a developer runs."
status: active
domain: draconic
area: toolchain
tags: [purpose]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Toolchain purpose

## Job

Give a developer one product that turns a Program into checked IR, JS or native artifacts, and interactive analysis: Compiler, Runtime, Embed, and CLI together.

## In scope

- **CLI**: `draconic parse`, `check`, `fmt`, `build`, `run`, and `repl` as the developer front of that product.
- **Frontend facade**: parse, bind, typecheck, and lower to IR; Script versus Module (link) policy lives here so callers do not wire stage crates.
- **Embed eval**: compile and evaluate source strings at run time on the native target.
- **LSP analysis**: diagnostics, hover types, and go-to-definition.

## Out of scope

- **Public Learn and Reference site content**: sources live under `website/`; this vault is not that site ([[0010-public-docs-draconic-ssg]], [[0013-public-site-tanstack-start]]).
- **Language semantics**: ECMA-262 meaning, Dual worlds, and Checker rules belong to the language spec slice, not this folder.
- **Package fetch and lock identity**: `get` / `mod tidy` / lock pins live under [[Packages purpose]].
- **Conformance fixture meaning**: `draconic test` and the harness live under [[Conformance purpose]].
- **Host I/O capability catalogue**: process, fs, sockets, and HTTP live under [[Host I/O purpose]].

## Surfaces

- **`draconic` binary**: parse, check, fmt, build `--target js|native`, run, repl `--target js|embed`.
- **Frontend API**: `compile_source` / `compile_path` as the compile entry.
- **Embed**: eval of expression strings inside the Runtime.
- **Language server**: diagnostics, hover, go-to-definition.

## Authority

- Behaviour: [[Toolchain — Contract]]
- Tests: [[Toolchain tests]]
- Glossary: [[CONTEXT]]
- Decisions: [[0001-rust-host-compiler]], [[0002-shared-ir-dual-backends]], [[0004-full-ecma-262-and-embed]], [[0010-public-docs-draconic-ssg]], [[0013-public-site-tanstack-start]]
- Shape: [[architecture-cli]], [[architecture-frontend]], [[architecture-embed]], [[architecture-lsp]]

## Open product questions

- (none)
