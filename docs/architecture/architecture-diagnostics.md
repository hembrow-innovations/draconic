---
id: "architecture-diagnostics"
title: "Frontend diagnostics"
kind: architecture
description: "Span, location, and diagnostic types shared by the Frontend crates."
domain: draconic
area: frontend
tags: [architecture, frontend, diagnostics]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Frontend diagnostics

## Overview

`draconic-diagnostics` is the shared location and error crate for the Frontend stack. Lexer, parser, checker, linker, and CLI all report failures as a single [[architecture-frontend|Frontend]] `Diagnostic` with a [[architecture-diagnostics#Design|half-open byte span]]. Pretty-print is rustc-style. Stable codes live under `codes` (Roadmap U09, done on [[ROADMAP]]).

## Context

The Compiler needs one span model from source bytes through parse and check, without each crate inventing its own line map. [[CONTEXT]] names the Frontend as parse, bind, typecheck, and lower; this crate is the diagnostic vocabulary those stages share. It does not own language rules.

## Design

### Crate layout

One source file. No submodules beyond `codes`.

- [x] **src/lib.rs**: public types, `codes`, unit tests
- [x] **Cargo.toml**: workspace crate; `thiserror` is declared, `Diagnostic` implements `std::error::Error` directly

### Public types

- **BytePos**: UTF-8 byte offset (`u32`) into a source buffer.
- **Span**: half-open `[start, end)`. `Span::new`, `dummy` (`0..0`), `is_dummy`, `len` (saturating). Dummy and zero-length spans skip the caret snippet in pretty output.
- **Location**: 1-based `line` and `column`. Column counts UTF-8 bytes from the start of the line, not Unicode scalars or display width.
- **SourceFile**: borrowed `name` plus `src`. `lookup` maps a `BytePos` to `Location` (offsets past EOF clamp to the last line, column = line length + 1). `line_text` returns a 1-based line without the trailing newline, or empty if out of range.
- **ErrorCode**: stable numeric code displayed as `E` plus four zero-padded digits (`ErrorCode(300)` → `E0300`).
- **Diagnostic**: `message`, `span`, optional `code`, optional `help`. Builders: `new`, `with_code`, `with_help`. `Display` is `message at start..end`, or `[E0NNN] message at start..end` when coded. `pretty(&SourceFile)` emits:

```text
error[E0300]: message
 --> name:line:col
  |
N | source line
  |     ^^^^
  = help: suggestion
```

Without a code the header is `error: message`. Help is omitted when unset.

### `codes` module (U09)

Assigned checker and host/FFI codes. Stable once assigned; tests lock labels.

- **E0300 NOT_ASSIGNABLE**: type not assignable to expected type
- **E0301 NOT_CALLABLE**: value is not callable
- **E0302 NOT_CONSTRUCTABLE**: value is not constructable with `new`
- **E0303 WRONG_ARITY**: wrong number of arguments
- **E0304 MISSING_RETURN**: annotated non-void function may fall off the end
- **E0305 EXCESS_PROPERTY**: object literal property absent from annotated shape
- **E0306 UNKNOWN_PROPERTY**: property name absent from annotated shape
- **E0307 INVALID_EXTERN_TYPE**: extern / FFI signature uses a non-ABI type or missing annotation (F06.02)
- **E0400 HOST_API_UNSUPPORTED**: host API not available on the compile target (H00.01)
- **E0401 EXTERN_UNSUPPORTED**: `extern "C"` / FFI native-only; unsupported on js (F08.01)
- **E0402 MISSING_DYNAMIC_LIB**: `--link` shared library path does not exist (F05.02)
- **E0403 POINTER_UNSUPPORTED**: native pointers (`*T`, `&x`, `*p = v`) unsupported on js (N04)

Lexer and parser diagnostics usually omit codes (message + span only).

### Tests

Inline `#[cfg(test)]` in `lib.rs`: span dummy/len, `Display`, code labels E0300–E0307 and E0400–E0403, `lookup` / `line_text`, pretty-print with caret, code+help, multiline, dummy/empty spans, two-digit gutter.

## Trade-offs

- **Single-line carets**: underline length is clamped to the remainder of the start line. Multi-line spans do not draw a second line.
- **Byte columns**: matches UTF-8 `Span` arithmetic used by [[architecture-lexer]] and [[architecture-parser]]; not grapheme-aware.
- **One diagnostic**: stages return `Result<_, Diagnostic>`, first failure wins. No warning stream, no related-span lists.

## Consequences

Every Frontend crate depends on this crate for `Span` and `Diagnostic`. [[architecture-ast]] stores spans on nodes. [[architecture-check]] attaches U09 codes. [[architecture-pipeline]] and backends reuse the same pretty-printer via `SourceFile`. Do not add a second span type.
