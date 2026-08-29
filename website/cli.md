---
title: CLI
section: reference
status: shipped
---

# CLI

The toolchain command is `draconic`. A Program is the unit of source it accepts.

- `draconic parse <file>` — parse a Program and print the AST dump
- `draconic build --target js|native <file> [-o <out>]` — compile to JavaScript or a native binary
- `draconic test <path>` — run conformance fixtures
- `draconic version` — print version
- `draconic help` — show usage

`parse` and `build` ship today. Package commands such as `get` and `mod tidy` are under [packages](reference-packages.html).

See also [types](types.html) and [Dual-world rules](dual-world-rules.html).
