---
id: "architecture-cli"
title: "CLI"
kind: architecture
description: "The draconic binary: parse, check, build, run, packages, and related developer commands."
domain: draconic
area: tooling
tags: []
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# CLI

## Overview

The Toolchain ships one binary, `draconic`, in the `draconic-cli` crate. It is the developer-facing front of [[architecture-pipeline]]: parse, check, format, document, extract, compile, run, REPL, conformance tests, git packages, and C header bindgen. Flag parsing is handwritten in `main.rs` and command modules. Command-line surface for humans also lives in [[api-cli]].

## Context

Callers should not wire Frontend, Linker, backends, or [[architecture-pkg]] themselves. The CLI owns argument parsing, toolchain-pin enforcement, package ensure-before-build, artifact paths, and process spawn. [[CONTEXT]] names this product the Toolchain, not “the compiler” alone.

## Design

Dispatch is a match on the first argument. Unknown tokens that look like a Program path (existing file, `.drac` / `.js`, or a path with a slash) invoke `run` so `#!/usr/bin/env draconic` works. `help` / `-h` / `--help` print usage. `version` / `-V` / `--version` print crate version, git commit, host triple, and detected LLVM.

Most commands except version and help call `toolchain_pin::enforce` on the input or workspace: optional pin mismatch warns; required pin mismatch exits 1. See [[architecture-pkg]].

Commands and flags that exist in code:

- **parse**: `draconic parse <file>`. Reads source, prints the parser AST dump. No extra flags.
- **extract**: `draconic extract <file>`. Parses as a Module and prints a v1 JSON extract (functions, classes, type aliases, externs, methods, constructors, accessors, imports, calls; an `exports` array is present but unused). No extra flags.
- **check**: `draconic check [--watch] <file>`. Frontend `check_path` (bind + typecheck, no emit). `--watch` polls mtime (`DRACONIC_WATCH_POLL_MS`, default 200 ms, minimum 10) and re-runs; errors print and the loop continues. There is no top-level `watch` command.
- **fmt**: `draconic fmt [--check] <file>`. Parse Script then Module, reprint with `print_program`, write in place. `--check` exits 1 when the file would change.
- **doc**: `draconic doc [--format md|html] [-o|--output <out>] <file>`. Extracts `/** … */` comments attached to the next declaration (`function`, `class`, `const` / `let` / `var`, including `export` / `async`). Default format is markdown to stdout. `--format` also accepts `markdown` and `htm`.
- **build**: `draconic build --target js|native [--watch] [--offline] [--strip|--strip-symbols] [--lto] [--link <lib.a>] <file> [-o|--out|--output <out>]`. `--target` is required (`--target=` accepted). Default output is `{stem}.js` or `{stem}` beside the input. Before compile, `ensure_locked_for_entry` materialises locked packages; `--offline` is cache-only (miss is a hard error). JS emit uses the JS backend. Native emit uses LLVM with optional LTO and extra `.a` archives. `--strip` and `--lto` are native-only; `--link` is native-only. `--strip` runs host `strip` (or `$STRIP`) and removes a companion `.dSYM` on macOS. `--watch` rebuilds on mtime change. Help text omits `--offline`; `parse_build_args` accepts it.
- **run**: `draconic run [--target js|native] [--allow-fs-read] [--allow-fs-write] [--allow-net-listen] [--allow-net-connect] <file> [args...]`. Default target is `js` (spawn `node`). Builds into a temp dir then executes; remaining tokens after the Program (or after `--`) are program argv. Allow flags exist in `parse_run_args` and, when any are present, set `DRACONIC_PERMISSIONS` to the grant subset (`fs-read`, `fs-write`, `net-listen`, `net-connect`). With no allow flags, Default permission policy is permissive ([[CONTEXT]], [[0008-host-io-sockets-first-http]]).
- **repl**: `draconic repl [--target js|embed]`. Default `js` compiles the accumulating session and runs Node, printing the last expression via `util.inspect`. `embed` evaluates each complete chunk through Embed. Multi-line continues while parse reports EOF. `.exit` and `.quit` leave the loop. No file argument.
- **test**: `draconic test [--coverage] [--jobs <n>] <path>`. Loads conformance fixtures (directory or one `.drac` plus optional `.meta`). `--coverage` reports JS line coverage (sequential). `--jobs` (or `DRACONIC_TEST_JOBS`) sizes a worker pool when coverage is off; default is 1 for a single fixture, otherwise available parallelism at least 2.
- **get**: `draconic get <module_path>@<ver> [--url <git-url>] [--dir <path>] [--cache-dir <path>]`. Adds or updates a git package; writes `draconic.toml` and `draconic.lock`; fetches into the module cache. See [[architecture-pkg]] and [[0009-go-style-git-packages]].
- **mod tidy**: `draconic mod tidy [--dir <path>] [--cache-dir <path>]`. Only `tidy` is a `mod` subcommand. Aligns lock with manifest, fetches missing, prunes unused.
- **bindgen**: `draconic bindgen <header> [-o|--output <out>]`. Parses a C header subset and writes Draconic `extern "C"` declarations. Default output is the header path with a `.drac` extension.

C header subset (F07): scalar and pointer functions, simple structs, typedef names. Maps C integers/floats to native types (`int` → `i32`, `void *` / `char *` → `*u8`, and so on). Skips `#` lines and comments. Rejects enum, union, bitfields, and function bodies. Not a full C preprocessor or clang-based bindgen.

Supporting modules: `c_header.rs` (library surface `draconic_cli::c_header`), `doc.rs`, `extract.rs`, `cmd_test.rs`, `strip_symbols.rs`, `toolchain_pin.rs`. `lib.rs` re-exports only `c_header`.

Build and run go through [[architecture-frontend]] `compile_path` / `check_path`, which choose parse versus [[architecture-linker]] then lower to IR and a backend ([[architecture-runtime]] is linked on the native path).

## Trade-offs

Hand-rolled flags keep the binary free of a CLI framework and make usage strings local to each command. Help can drift from parsers (`--offline` on build). Watch is a poll loop, not fs-events, so it stays simple and testable. Run builds a fresh artifact every time rather than caching. Bindgen is a small header subset, not libclang.

## Consequences

Package identity and lock pins are [[architecture-pkg]] concerns; the CLI only parses get/tidy/build-offline and prints results. Editors do not talk to this binary for analysis; that is [[architecture-lsp]]. Permission grants apply only to `run` (and shebang-as-run), not to `build` or `check`.
