---
title: CLI
section: reference
status: shipped
---

# CLI

The toolchain command is `draconic`. A Program is the unit of source it accepts. Toolchain means the whole product a developer runs: Compiler, Runtime, Embed, and CLI together.

A session that ships today:

```
draconic parse hello.drac
draconic check hello.drac
draconic build --target js hello.drac -o hello.js
draconic run hello.drac
```

`hello.drac` can be a small Program:

```drac
let console = globalThis.console;
console.log("hello from Draconic");
```

## Commands

- `draconic parse <file>` — parse a Program and print the AST dump
- `draconic check [--watch] <file>` — typecheck with no emit
- `draconic fmt [--check] <file>` — format in place (`--check` reports whether the file is already formatted)
- `draconic build --target js|native [--strip] [--lto] <file> [-o <out>]` — compile to JavaScript or a native binary. `--strip` and `--lto` are native-only size opts (LTO is a size-delta smoke versus the default native artifact).
- `draconic run [--target js|native] [--allow-fs-read] [--allow-fs-write] [--allow-net-listen] [--allow-net-connect] <file> [args...]` — build and execute a Program. Default target is js, which executes the emitted JavaScript with `node` on PATH.
- `draconic repl [--target js|embed]` — interactive loop. `embed` is the native eval path.
- `draconic test <path>` — run Conformance fixtures, not a general application test runner
- `draconic version` — print version
- `draconic help` — show usage

Package commands such as `get` and `mod tidy` are under [packages](reference-packages.html). `bindgen` is documented in the repository README.

## Permissions

Default permission policy is permissive. A Program run with no `--allow-*` flags may use the filesystem and TCP surfaces the host already exposes. This is not a Deno deny-by-default. The `--allow-fs-read`, `--allow-fs-write`, `--allow-net-listen`, and `--allow-net-connect` flags on `draconic run` install an opt-in grant subset when you want one.

## Shebang

`#!/usr/bin/env draconic` invokes `run` on that Program path. In-repo example: `examples/shebang/hello.drac`. Make it executable and run it when `draconic` is on `PATH`:

```bash
chmod +x examples/shebang/hello.drac
./examples/shebang/hello.drac
```

See also [types](types.html) and [Dual-world rules](dual-world-rules.html).
