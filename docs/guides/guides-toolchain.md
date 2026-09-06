---
id: guides-toolchain
title: Using the toolchain
kind: guide
description: Install, parse, build JavaScript or native, run a Program, and use in-repo examples.
domain: draconic
area: guides
tags: [guide, toolchain, cli]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Using the toolchain

## Overview

This guide is the developer flow for the `draconic` CLI: install, parse a Program, build to JavaScript or a native binary, run it, and walk `examples/`. Architecture of the CLI is [[architecture-cli]]. Command flags are [[api-cli]]. Packages are [[architecture-pkg]] and [[0009-go-style-git-packages]]. Host I/O is [[0008-host-io-sockets-first-http]]. Terms are in [[CONTEXT]]. Completeness is [[ROADMAP]].

## Prerequisites

- **PATH**: after the install script, `~/.draconic/bin` must be on `PATH` so `draconic` resolves.
- **Rust (`cargo`)**: required to build the Toolchain from a clone.
- **LLVM**: required for `--target native` (native binaries and the website generator). The JS target does not need LLVM.
- **Node**: `draconic run` defaults to `--target js` and executes the emitted JS with `node`.
- **Git**: required for `draconic get` / `draconic mod tidy` (git-backed packages).

Native builds fail without an LLVM toolchain on the machine. That is expected, not a JS fallback.

## Steps

1. **Install a binary** (or skip to step 2 for a clone). Run the public install script, then confirm the CLI:

   `curl -fsSL https://raw.githubusercontent.com/hembrow-innovations/draconic/main/scripts/install.sh | sh`

   Then `draconic -V` and `draconic parse hello.drac`.

2. **Or build from source**. Clone the repo, then `cargo build -p draconic-cli --release`. During development, `cargo run -p draconic-cli -- <subcommand> …`. Prefer `cargo test --workspace` over ad-hoc scripts.

3. **Write a Program**. Save a `.drac` file. A Program is the unit of source the Toolchain accepts ([[CONTEXT]]).

4. **Parse**. `draconic parse hello.drac` prints the AST dump. Failures exit non-zero.

5. **Check (optional)**. `draconic check hello.drac` typechecks and binds with no emit. `--watch` re-runs on change.

6. **Build JavaScript**. `draconic build --target js hello.drac -o hello.js`. Run the artifact with `node hello.js` if you want the file on disk.

7. **Run**. `draconic run hello.drac` builds and executes (default target `js`). Pass `--target native` for the LLVM path. Remaining args after the file go to the Program.

8. **Build native** when LLVM is present. `draconic build --target native hello.drac -o hello` then `./hello`. `--strip` and `--lto` are native-only.

9. **Packages (optional)**. `draconic get <module_path>@<ver>` then `draconic mod tidy`. `draconic build` auto-fetches missing locked deps unless `--offline` ([[0009-go-style-git-packages]]).

10. **Format and test**. `draconic fmt [--check] <file>`. `draconic test [--coverage] [--jobs <n>] <path>` runs Conformance fixtures.

## Examples

Minimal Program (`hello.drac`):

```js
let console = globalThis.console;
console.log("hello from Draconic");
```

Parse, JS build, run:

```bash
draconic parse hello.drac
draconic build --target js hello.drac -o hello.js
node hello.js
draconic run hello.drac
```

Native binary (LLVM required):

```bash
draconic build --target native hello.drac -o hello
./hello
```

Shebang: `#!/usr/bin/env draconic` invokes `run` on the script path (`examples/shebang/`).

In-repo examples:

- **examples/fizzbuzz/**: control flow and strings, compiled to JS
- **examples/http-echo/**: native HTTP/1.1 listen and accept ([[0008-host-io-sockets-first-http]])
- **examples/todo/**: browser todo via `globalThis`
- **examples/pkg-lib/** and **examples/pkg-consumer/**: git packages (`draconic.toml`, `get` / `mod tidy`)
- **examples/shebang/**: chmod and execute
- **examples/flagship-service/**: larger native-shaped example when present

From a clone, equivalent CLI via Cargo:

```bash
cargo run -p draconic-cli -- run hello.drac
cargo run -p draconic-cli -- build --target js hello.drac -o hello.js
cargo run -p draconic-cli -- test tests/conformance/fixtures
cargo test --workspace
```

## Reference

- **CLI contract**: [[api-cli]]
- **CLI architecture**: [[architecture-cli]]
- **Embed eval**: [[api-embed]], [[architecture-embed]]
- **Packages**: [[architecture-pkg]], [[0009-go-style-git-packages]]
- **Host I/O default policy**: [[security]], [[0008-host-io-sockets-first-http]]
- **Public Learn/Reference**: [[guides-public-docs]]
- **Language Loop**: [[guides-loop]]
