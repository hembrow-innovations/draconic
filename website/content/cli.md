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

## parse

`draconic parse <file>` — parse a Program and print the AST dump

## extract

`draconic extract <file>` — print v1 JSON for functions, types, imports, and calls in one Program

## check

`draconic check [--watch] <file>` — typecheck with no emit. `--watch` re-runs when the file changes.

## fmt

`draconic fmt [--check] <file>` — format in place (`--check` reports whether the file is already formatted)

## doc

`draconic doc [--format md|html] [-o <out>] <file>` — extract `/** doc comments */` to markdown (default) or HTML. Writes stdout unless `-o` names a file.

Save this as `greet.drac`. It builds today:

```drac
let console = globalThis.console;

/** Greet a name. */
function greet(name: string): string {
  return "hello " + name;
}

console.log(greet("from Draconic"));
```

```
draconic check greet.drac
draconic doc greet.drac
```

## build

`draconic build --target js|native [--watch] [--strip] [--lto] [--link <lib.a>] <file> [-o <out>]` — compile to JavaScript or a native binary. `--target` is required. `--strip` and `--lto` are native-only size opts (LTO is a size-delta smoke versus the default native artifact). `--link` is native-only. `--watch` rebuilds on change. When `-o` is omitted, JS writes `{stem}.out.js` and native writes `{stem}.out` beside the input.

## run

`draconic run [--target js|native] [--allow-fs-read] [--allow-fs-write] [--allow-net-listen] [--allow-net-connect] <file> [args...]` — build and execute a Program. Default target is js, which executes the emitted JavaScript with `node` on PATH. Leftover args after the file are `processArgs()`.

## repl

`draconic repl [--target js|embed]` — interactive loop. Default is js. `embed` is the native eval path. Multi-line continues until the chunk parses. Type `.exit` or `.quit` to leave.

## test

`draconic test [--coverage] [--jobs <n>] <path>` — run Conformance fixtures, not a general application test runner. `<path>` is a directory or a `.drac` file. `--coverage` reports JS line coverage. `--jobs` sets the worker pool when coverage is off.

## version

`draconic version` — print version

## help

`draconic help` — show usage

Package commands such as `get` and `mod tidy` are under [packages](reference-packages.html).

## bindgen

`draconic bindgen <header> [-o <out>]` — write Draconic `extern "C"` declarations from a C header. Default output is the header path with a `.drac` extension.

## Permissions

Default permission policy is permissive. A Program run with no `--allow-*` flags may use the filesystem and TCP surfaces the host already exposes. This is not a Deno deny-by-default. The `--allow-fs-read`, `--allow-fs-write`, `--allow-net-listen`, and `--allow-net-connect` flags on `draconic run` install an opt-in grant subset when you want one.

## Shebang

`#!/usr/bin/env draconic` invokes `run` on that Program path. In-repo example: `examples/shebang/hello.drac`. Make it executable and run it when `draconic` is on `PATH`:

```bash
chmod +x examples/shebang/hello.drac
./examples/shebang/hello.drac
```

See also [types](types.html) and [Dual-world rules](dual-world-rules.html).
