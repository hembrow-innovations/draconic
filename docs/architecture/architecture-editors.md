---
id: "architecture-editors"
title: "Editors"
kind: architecture
description: "Zed syntax highlighting for .drac files; no LSP process and no other editor trees."
domain: draconic
area: tooling
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Editors

## Overview

The repo ships one editor extension: Zed, under `editors/zed`. It highlights `.drac` files using the TypeScript tree-sitter grammar plus Draconic native types and `extern`. It does not start [[architecture-lsp]] or [[architecture-cli]]. There is no VS Code, Neovim, or other editor package in this tree.

## Context

Draconic source is an ECMAScript / TypeScript-shaped superset (`.drac`). Editors need at least comments, brackets, and type names before a language server exists. Reusing tree-sitter TypeScript avoids a second grammar while the language is still moving. A full LSP host is explicitly later; [[architecture-lsp]] is a library, not a server.

## Design

`editors/zed/extension.toml` identifies the extension as `draconic` (syntax highlighting for `.drac`). Grammar `draconic` is fetched from `tree-sitter/tree-sitter-typescript` (pinned rev, `path = "typescript"`). First install compiles that grammar (needs network). README: install as a Zed **dev extension** pointing at `editors/zed` (the directory that contains `extension.toml`), not the repo root.

Language config (`languages/draconic/config.toml`): name Draconic, suffix `drac`, `//` line comments, `/* */` blocks, typical brackets, tab size 2, word characters `#` and `$`. Query files beside it: highlights, indents, brackets, outline, overrides.

Highlights treat `i8` `i16` `i32` `i64` `u8` `u16` `u32` `u64` `f32` `f64` `bool` as builtin types and `extern` as a keyword, on top of TypeScript identifier/function/class queries.

No `language_servers` key in `extension.toml`. Hover, diagnostics, and go-to-definition from [[architecture-lsp]] are unused here.

## Trade-offs

TypeScript grammar coverage is high for JS-like syntax and wrong or incomplete for Draconic-only forms until queries catch them. Dev-extension install is manual. A vendored `grammars/draconic` copy of tree-sitter-typescript is large relative to the language queries.

## Consequences

Toolchain behaviour stays in [[architecture-cli]], [[architecture-pkg]], and [[architecture-linker]]. Improving editor intelligence means wrapping the LSP analysis library in a server, then registering it in this extension — not growing a second Checker inside Zed queries.
