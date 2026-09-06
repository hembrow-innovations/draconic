---
id: "architecture-lexer"
title: "Frontend lexer"
kind: architecture
description: "Token kinds, span model, trivia, and regexp early errors for Draconic source."
domain: draconic
area: frontend
tags: [architecture, frontend, lexer]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Frontend lexer

## Overview

`draconic-lexer` scans UTF-8 source into a `Vec<Token>` (Roadmap B01, done). Tokens carry [[architecture-diagnostics|Span]] plus flags the [[architecture-parser|parser]] needs for ASI, restricted productions, and strict-mode legacy octal. There is no error recovery: the first invalid character, unterminated comment, or invalid regexp literal returns a `Diagnostic`.

## Context

ECMA-262 requires context-sensitive `/` (division vs RegularExpressionLiteral), template interpolations nested with braces, hashbang only at byte 0, and Annex B HTML-like comments in Script but not Module. The lexer owns those scans so the parser sees a flat token stream. [[CONTEXT]] Program input is a file or string; this crate does not read the filesystem.

## Design

### Crate layout

- [x] **src/lib.rs**: `JsString`, `Token`, `TokenKind`, `Lexer`, `tokenize`, unit tests
- [x] **src/regexp.rs**: `validate_regexp_literal`, `validate_regexp_flags`, pattern checks via `regress`
- [x] **Cargo.toml**: depends on `draconic-diagnostics`, `regress`, `unicode-id-start`

### Span model

Each `Token.span` is a half-open UTF-8 byte range from [[architecture-diagnostics]]. Trivia (whitespace, comments, hashbang, HTML comments) is not tokenized; it only sets `preceded_by_line_terminator` on the next token.

### Token flags

- **preceded_by_line_terminator**: a LineTerminator was skipped immediately before this token (postfix `++`/`--`, `continue`/`break`/`return`/`throw`, ASI).
- **escaped**: identifier or keyword contained a Unicode escape (`\u…`). Contextual keywords (`get`/`set`/`async`) must not be escaped (E19.39); the parser enforces that.
- **legacy_octal**: Annex B legacy octal / NonOctalDecimal numeric or string escape (E19.69). Strict mode (and always templates) reject these as early SyntaxError.

### TokenKind

Punctuators: `LParen` `RParen` `LBrace` `RBrace` `LBracket` `RBracket` `Semi` `Comma` `Dot` `DotDotDot` `Colon` `At` `Question` `QuestionDot` `QuestionQuestion` `QuestionQuestionEq`.

Operators: `Plus` `PlusPlus` `PlusEq` `Minus` `MinusMinus` `MinusEq` `Star` `StarStar` `StarStarEq` `StarEq` `Slash` `SlashEq` `Percent` `PercentEq` `Bang` `Eq` `EqEq` `EqEqEq` `Arrow` `NotEq` `NotEqEq` `Lt` `LtEq` `Gt` `GtEq` `AndAnd` `AndAndEq` `OrOr` `OrOrEq` `BitAnd` `BitAndEq` `BitOr` `BitOrEq` `BitXor` `BitXorEq` `Tilde` `Shl` `ShlEq` `Shr` `ShrEq` `UShr` `UShrEq`.

Atoms and keywords: `Ident(String)` `PrivateIdent(String)` `Number(String)` `BigInt(String)` `String(JsString)` `TemplateNoSubstitution` `TemplateHead` `TemplateMiddle` `TemplateTail` `True` `False` `Null` `Let` `Const` `Var` `TypeOf` `Void` `Delete` `If` `Else` `While` `Do` `For` `Break` `Continue` `Switch` `Case` `Default` `In` `InstanceOf` `Of` `Function` `Async` `Await` `Yield` `Return` `This` `New` `Class` `Extends` `Super` `Static` `Throw` `Try` `Catch` `Finally` `With` `Import` `Export` `From` `As` `RegExp { pattern, flags }` `Eof`.

`Number` / `BigInt` keep canonical source text (including `n`). Strings and template quasis are `JsString` (UTF-16 code units, unpaired surrogates allowed; `to_string_lossy` for dumps).

### Lexer API

- **Lexer::new**: Script goal. Annex B HTML comments allowed (`<!--` anywhere as open comment; `-->` at line start as close comment).
- **Lexer::new_module**: Module goal (E19.67). HTML-like comments disabled.
- **tokenize**: skip hashbang (`#!` only at absolute start), then loop `next_token` until `Eof`. After each non-EOF token, `allow_regexp` is set from `regexp_allowed_after` so `/` after an ident, number, `)`, `]`, `++`, `--`, etc. is division.

Other scanner state: template `${…}` brace depth stack; `at_line_start` for HTML close comments; `pending_legacy_octal` consumed by `finish_token`.

Trivia skipped: TAB/VT/FF/SP, CR/LF/CRLF, LS/PS, Unicode Space_Separator / NBSP / BOM, `//` line comments, `/* */` block comments (unterminated → diagnostic). Hashbang is not a token.

`QuestionDot` is not emitted when `?.` is followed by a decimal digit (`x?.3:y` stays `Question` then number).

### Regexp module

`validate_regexp_literal(pattern, flags)` is the early-error hook for `/pattern/flags`. Flags allowed: `d g i m s u v y`, each at most once; `u` and `v` exclusive. Pattern parse uses `regress` with flag-dependent grammar. `\p{Script=Unknown}` / `Zzzz` (and Script_Extensions aliases) are rewritten to `Latin` for regress so UTS#24 special values still validate.

### Tests

- **lib.rs tests**: tokens, ASI flags, templates, regexp vs division, hashbang, HTML comments Script vs Module, legacy octal, ident escapes, numeric separators, early errors such as `1.toString` / `3in`.
- **regexp.rs tests**: flags ok/reject, `u`+`v`, invalid patterns, Unicode property escapes, Script Unknown/Zzzz rewrite.

No `tests/` directory. No fuzz crate on the lexer; parser fuzz drives tokenize indirectly.

## Trade-offs

- **Fail fast**: no skip-and-continue after a bad character. Matches parser `Result` and keeps snapshots simple.
- **HTML comments in Script only**: Module uses `new_module` so E19.67 is a lexer fact, not a parser post-pass.
- **regress for Pattern**: not a hand-written regexp grammar; Script Unknown is a local rewrite, not a full UTS#24 engine.

## Consequences

[[architecture-parser]] is the only in-tree consumer of `tokenize` for Programs. [[architecture-ast]] re-exports `JsString` for string and template nodes. Do not scan source in the checker or IR. Callers that need Module HTML-comment rejection must use `Lexer::new_module`, which `parse_module` already does.
