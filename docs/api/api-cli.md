---
id: api-cli
title: draconic CLI
kind: api
description: Real draconic subcommands and flags from print_usage and parse_*_args. Not an OpenAPI spec.
domain: draconic
area: api
tags: [api, cli]
source: crates/draconic-cli/src/main.rs
created_at: "2026-09-06"
updated_at: "2026-09-12"
---

# draconic CLI

## Overview

`draconic` is the Toolchain command. A Program is the unit of source it accepts ([[CONTEXT]]). This note is the CLI contract from `print_usage` and `parse_*_args` in `crates/draconic-cli/src/main.rs` (plus `cmd_test.rs` `parse_test_args` and `extract.rs` `cmd_extract`). It is not OpenAPI. Architecture: [[architecture-cli]]. Packages: [[architecture-pkg]], [[0009-go-style-git-packages]]. Host grants: [[0008-host-io-sockets-first-http]], [[security]]. How to run it: [[guides-toolchain]].

Endpoints below are commands. Global: `draconic help` / `-h` / `--help` print usage. `draconic version` / `-V` / `--version` print verbose version (commit, host, LLVM). Unknown commands exit 2. A bare script path (`*.drac` / `*.js` / existing file / path with a slash) invokes `run` (shebang: `#!/usr/bin/env draconic`).

## Endpoints

- **parse `<file>`**: parse a Program and print the AST dump. No extra flags. Usage error if the file is missing.

- **extract `<file>`**: print v1 JSON extract for one Program (`extract.rs`). No extra flags.

- **check `[--watch] <file>`**: typecheck and bind, no emit (`parse_check_args`). `--watch` re-runs on change.

- **fmt `[--check] <file>`**: format in place. `--check` exits 1 when the file is not already formatted (no write).

- **doc `[--format md|html] [-o <out>] <file>`**: extract `/** doc comments */` to markdown (default) or HTML. `-o` / `--output` writes a file; otherwise stdout. Unknown `--format` exits 2.

- **build `--target js|native` `[--watch] [--offline] [--library] [--strip] [--lto] [--link <lib.a>] <file> [-o <out>]`**: compile to JS or a native binary. `--target` is required (`js` or `native`). `parse_build_args` also accepts `--target=`, `-o` / `--out` / `--output` / `--out=` / `--output=`, `--watch`, `--offline` (cache-only package ensure; miss is a hard error), `--library` (js only: named ESM exports from IR metadata), `--strip` / `--strip-symbols` (native only), `--lto` (native only), `--link` / `--link=` extra `.a` (native only). `print_usage` omits `--offline`; the parser accepts it. `--strip` or `--lto` with `--target js` is an error. `--library` with `--target native` is an error. When `-o` is omitted, JS is `{stem}.out.js` and native is `{stem}.out` beside the input (gitignored scratch names; [[architecture-cli]]).

- **run `[--target js|native] [--allow-fs-read] [--allow-fs-write] [--allow-net-listen] [--allow-net-connect] <file> [args...]`**: build and execute. Default target `js`. `parse_run_args`: `--target` / `--target=`, the four `--allow-*` flags, `--` then program argv, then remaining tokens after the file as program argv. Non-empty grants set `DRACONIC_PERMISSIONS` on the child. Unflagged run is permissive ([[security]]).

- **repl `[--target js|embed]`**: interactive REPL (`parse_repl_args`). Default `js`. `--target embed` uses Embed eval ([[api-embed]], [[architecture-embed]]). `.exit` / `.quit` end the session. Multi-line when parse hits EOF.

- **test `[--coverage] [--jobs <n>] <path>`**: Conformance fixtures (`parse_test_args`). `--coverage` reports JS line coverage. `--jobs` / `--jobs=` worker pool size (`>= 1`). `<path>` is a directory or a `.drac` file.

- **get `<module_path>@<ver> [--url <git-url>] [--dir <path>] [--cache-dir <path>]`**: add or update a git package, fetch, write lock (`parse_get_args`). `--url` / `--url=`, `--dir` / `--dir=`, `--cache-dir` / `--cache-dir=`.

- **mod tidy `[--dir <path>] [--cache-dir <path>]`**: only `tidy` is a mod subcommand (`parse_mod_tidy_args`). Align lock with manifest; fetch missing; prune unused.

- **bindgen `<header> [-o <out>]`**: write Draconic `extern "C"` decls from a C header. `-o` / `--output`. Default output path comes from the header name when `-o` is omitted.

## Auth

The CLI has no user login. `--allow-*` on `run` are host permission grants, not identity ([[0008-host-io-sockets-first-http]]). Private git credentials for `get` / tidy / build fetch come from the environment and must not be stored in `draconic.toml` or `draconic.lock` ([[security]], [[0009-go-style-git-packages]]).

## Errors

- **Exit 2**: usage / unknown command / unknown option / missing required args.
- **Exit 1**: read/parse/check/build/run failure; `fmt --check` would reformat; package or test failures; `bindgen` parse/write errors.
- **Exit 0**: success. `run` forwards the child exit code when it is in 1..=255.
- **Diagnostics**: toolchain failures print `error: …` (or command-specific prefixes such as `bindgen:`). Package integrity and resolve failures are hard errors ([[reliability]]). Native-only flags on js are rejected at parse (`--strip`, `--lto`).
