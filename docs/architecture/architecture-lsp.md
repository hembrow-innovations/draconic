---
id: "architecture-lsp"
title: "LSP"
kind: architecture
description: "Source-buffer analysis library for diagnostics, hover, and go-to-definition — not a language server process."
domain: draconic
area: tooling
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# LSP

## Overview

`draconic-lsp` is an analysis library (Roadmap U06). It takes a source string, runs Frontend check, and answers diagnostics, hover types, and go-to-definition. It is not a `tower-lsp` server and not a [[architecture-cli]] subcommand. Editor hosts are expected to own JSON-RPC later.

## Context

Editors need bind and Checker results without emit. Shipping a full LSP process would duplicate Frontend wiring and freeze a protocol surface before the analysis API is stable. The crate exposes that API as `Analysis` so [[architecture-cli]] or [[architecture-editors]] can wrap it when a server exists.

## Design

`Analysis::analyze` / `analyze` call `draconic_frontend::check_source` (Script: parse, bind, typecheck, no emit, no [[architecture-linker]] graph). Success stores a `CheckedProgram`. Failure stores one `LspDiagnostic` (message + span) and no checked program.

- **diagnostics**: empty when check succeeds; otherwise the Frontend diagnostic. `start_location` maps the span start through `SourceFile::lookup` (1-based line/column).
- **hover** (`offset` is a UTF-8 byte): prefers a binding use, then a declaration name, then the smallest typed expression containing the offset. `type_string` comes from `CheckedProgram::format_type`. Returns none when check failed.
- **goto_definition**: from a use, the symbol’s declaration span; on a declaration, the same span. None when check failed or the offset is not an identifier.

Helpers: `offset_to_location`, `location_to_offset` (1-based line/column, column counts UTF-8 bytes, out of range clamps to EOF).

Types: `LspDiagnostic`, `Hover`, `Definition`, `Analysis`.

## Trade-offs

A library avoids a long-lived server, workspace indexing, and incremental parse. It also cannot yet serve multi-file Modules, package imports, or pull diagnostics from a linked graph. One diagnostic per failed check matches current Frontend error reporting, not a full diagnostic list.

## Consequences

[[architecture-editors]] (Zed) does not load this crate; highlighting is tree-sitter only. A future LSP process should call this API rather than reimplementing check. Pipeline stages remain in [[architecture-frontend]]; this crate must not grow a second Checker.
