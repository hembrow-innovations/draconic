# Draconic

**JavaScript you already know. Native types when you need them. One language, two backends.**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Docs](https://img.shields.io/badge/docs-Learn%20%26%20Reference-0A7EA4)](https://hembrow-innovations.github.io/draconic/)
[![GitHub](https://img.shields.io/badge/github-hembrow--innovations%2Fdraconic-181717?logo=github)](https://github.com/hembrow-innovations/draconic)

Draconic is a **full ECMAScript superset** with TypeScript-inspired static types and unboxed **native systems types**. Compile the same Program to **JavaScript** or to a **native binary** via LLVM.

> **Status.** Early (v0.1). The toolchain can parse, typecheck, build (`js` | `native`), and test today. Language completeness is still growing — see [ROADMAP.md](./ROADMAP.md).

## Why Draconic

Write Programs that look like JavaScript. Add `i32`, `i64`, and fixed structs when you want values off the GC heap. JS values and native types coexist in one Program at explicit boundaries — **Dual worlds** — instead of a typed-JS-only story or a separate FFI language.

- **From JavaScript / TypeScript** — a familiar surface; the JS backend emits JavaScript, not TypeScript
- **From systems** — LLVM + a tracing GC Runtime so ECMAScript semantics still hold next to native types

## Features

- Full ECMAScript superset (not a subset or a new syntax)
- TypeScript-inspired Checker (familiar surface; not tsc-compatible)
- Native types (`i32`, `i64`, fixed structs) outside the GC heap
- Two backends: JavaScript emit, and LLVM native binaries
- Host I/O on native: process, stdio, sockets, then thin HTTP/1.1
- Git-backed packages (Go-style module paths; no central registry required)
- ESM modules, `draconic fmt`, `check`, `run`, `repl`, and `test`

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/hembrow-innovations/draconic/main/scripts/install.sh | sh
```

Places `draconic` in `~/.draconic/bin`. Add that directory to `PATH` if needed:

```bash
draconic -V
draconic parse hello.drac
```

Native builds need an LLVM toolchain on the machine.

## Quick start

Save this as `hello.drac`:

```js
let console = globalThis.console;
console.log("hello from Draconic");
```

Parse, build to JavaScript, or run it:

```bash
draconic parse hello.drac
draconic build --target js hello.drac -o hello.js
node hello.js

draconic run hello.drac
```

Native binary:

```bash
draconic build --target native hello.drac -o hello
./hello
```

A native HTTP/1.1 echo server (no C host) is a few lines:

```js
let s = tcpListen(8080);
stdoutWrite("listening on 8080\n");
while (true) {
  let a = tcpAccept(s);
  let raw = tcpRead(a, 65536);
  let req = httpParseRequest(raw);
  let resp = httpWriteResponse(200, "OK", "Content-Type: text/plain\r\n", req.path);
  tcpWrite(a, resp);
  closeTcp(a);
}
```

## Examples

| Example | What it shows |
|---------|----------------|
| [`examples/fizzbuzz/`](./examples/fizzbuzz/) | Control flow and strings, compiled to JS |
| [`examples/http-echo/`](./examples/http-echo/) | Native HTTP/1.1 listen and accept |
| [`examples/todo/`](./examples/todo/) | Browser todo via `globalThis` (DOM, `localStorage`) |
| [`examples/pkg-lib/`](./examples/pkg-lib/) · [`pkg-consumer/`](./examples/pkg-consumer/) | Git packages (`draconic.toml`, `get` / `mod tidy`) |

## Documentation

- **[Learn](https://hembrow-innovations.github.io/draconic/)** — from JavaScript or from systems, then Dual worlds, modules, native types, host I/O, packages
- **[Reference](https://hembrow-innovations.github.io/draconic/reference.html)** — CLI, types, Dual-world rules, host I/O, packages
- **[Install](https://hembrow-innovations.github.io/draconic/install.html)** — public install path

Sources for those pages live in [`website/`](./website/).

## CLI

```text
draconic parse <file>                 Parse and print the AST
draconic check [--watch] <file>       Typecheck (no emit)
draconic fmt [--check] <file>         Format in place
draconic build --target js|native <file> [-o <out>]
draconic run [--target js|native] <file> [args...]
draconic repl [--target js|embed]
draconic test <path>                  Conformance fixtures
draconic get <module_path>@<ver>      Add a git package
draconic mod tidy                     Align lockfile and fetch
draconic bindgen <header>             extern "C" from a C header
draconic version | help
```

Shebang: `#!/usr/bin/env draconic`

## Build from source

Requires a Rust toolchain (`cargo`). LLVM is required for `--target native`.

```bash
git clone https://github.com/hembrow-innovations/draconic.git
cd draconic
cargo build -p draconic-cli --release
cargo test --workspace
```

During development:

```bash
cargo run -p draconic-cli -- run hello.drac
cargo run -p draconic-cli -- build --target js hello.drac -o hello.js
cargo run -p draconic-cli -- test tests/conformance/fixtures
```

The workspace is a Rust monorepo: lexer → parser → check → IR → JS / LLVM backends, plus Runtime, Embed, packages, and the `draconic` CLI under `crates/`.

## Contributing

Draconic is built in the open. Completeness is the [Roadmap](./ROADMAP.md): a feature is done only when its tests are green on every applicable target.

```bash
cargo test --workspace
```

Start from a `todo` row, add or extend tests first, then implement. Domain terms are in [CONTEXT.md](./CONTEXT.md); locked decisions are in [docs/adr/](./docs/adr/). Pull requests are welcome.

## License

[MIT](https://opensource.org/licenses/MIT)
