# Draconic

Draconic is a programming language: a **full ECMAScript superset** with TypeScript-inspired static types and **native systems types**, compiling to **JavaScript** and to **native binaries** (LLVM).

This monorepo is the Draconic toolchain (Rust) plus the agent **Loop** that grows the language to completion.

## Status

Bootstrap / Roadmap Loop spine is complete. Language completeness is ongoing — see [`ROADMAP.md`](./ROADMAP.md) for open items. The toolchain is usable today for **parse**, **build** (`js` | `native`), and **test**.

## Build

```bash
cargo build -p draconic-cli
cargo test --workspace
```

## Quick start

Write a program (`hello.drac`):

```js
let console = globalThis.console;
console.log("hello");
```

Parse (AST dump):

```bash
cargo run -p draconic-cli -- parse hello.drac
```

Build to JavaScript:

```bash
cargo run -p draconic-cli -- build --target js hello.drac -o hello.js
node hello.js
```

Native binary (needs LLVM toolchain):

```bash
cargo run -p draconic-cli -- build --target native hello.drac -o hello
./hello
```

Examples: [`examples/fizzbuzz/`](./examples/fizzbuzz/) (CLI FizzBuzz) · [`examples/http-echo/`](./examples/http-echo/) (native HTTP/1.1) · [`examples/todo/`](./examples/todo/) (browser todo).

## CLI

```text
draconic parse <file>                          Parse a Program and print the AST dump
draconic build --target js|native <file> [-o <out>]
                                               Compile a Program to JS or a native binary
draconic test <path>                           Run conformance fixtures (dir or .drac file)
draconic version                               Print version
draconic help                                  Show this help
```

Via cargo:

```bash
cargo run -p draconic-cli -- help
cargo run -p draconic-cli -- parse path/to/file.drac
cargo run -p draconic-cli -- build --target js path/to/file.drac -o out.js
cargo run -p draconic-cli -- build --target native path/to/file.drac -o out
cargo run -p draconic-cli -- test tests/conformance/fixtures
cargo run -p draconic-cli -- version
```

## Layout

| Path | Role |
|------|------|
| `crates/draconic-lexer` | Lexer |
| `crates/draconic-parser` | Parser (source → AST) |
| `crates/draconic-linker` | ESM module linker |
| `crates/draconic-frontend` | Frontend (parse/link → check → IR) |
| `crates/draconic-ast` | AST + dump |
| `crates/draconic-check` | Binder + Checker |
| `crates/draconic-ir` | Shared IR |
| `crates/draconic-backend-js` | JS backend |
| `crates/draconic-backend-llvm` | LLVM backend |
| `crates/draconic-runtime` | Native Runtime (GC, async) |
| `crates/draconic-embed` | Embed (eval-at-runtime) |
| `crates/draconic-cli` | `draconic` CLI |
| `tests/conformance` | Conformance suite |
| `examples/` | Example programs (`fizzbuzz`, `http-echo`, `todo`) |
| `tests/test262` | Staged Test262 harness (optional suite via `scripts/fetch-test262.mjs`) |
| `ROADMAP.md` | Feature checklist (Loop source of truth) |
| `CONTEXT.md` | Domain glossary |
| `docs/adr/` | Architecture decisions |
| `.agents/skills/draconic-loop/` | Mega-loop skill |

## Agent loop

Invoke the **draconic-loop** skill to claim the next Roadmap item and implement it test-first (one item per session by default).

Or run N iterations unattended (same pattern as life-engine):

```bash
# OpenCode TUI — /loop (default 100×) or /loop 20
# (see .opencode/command/loop.md)

# OpenCode CLI — default prompt = one draconic-loop each iteration
node .loop/opencode-loop.mjs 10

# Optional sleep between loops (seconds)
SLEEP=30 node .loop/opencode-loop.mjs 10

# Stall watchdog: kill a hung iteration after N seconds with no stdout (default 600)
STALL_SEC=900 STALL_ACTION=continue node .loop/opencode-loop.mjs 100

# Custom prompt / model flags after --
node .loop/opencode-loop.mjs 5 "run the draconic-loop skill once" -- -m xai/grok-4.5

# Claude Code
node .loop/claude-loop.mjs 10

# One swarm wave (N fresh sessions; default serial on main worktree)
node .loop/opencode-swarm.mjs wave=10
# Parallel: one git worktree per slot under .loop/worktrees/; always removed after
node .loop/opencode-swarm.mjs parallel wave=10

# Until ROADMAP todo=0 (outer loop holds no LLM context)
node .loop/opencode-orchestrate.mjs wave=10
node .loop/opencode-orchestrate.mjs parallel wave=10 MAX_WAVES=50

# TUI: /swarm  |  /orchestrate
# Status / worktree hygiene:
node .loop/roadmap-status.mjs
node .loop/worktree.mjs list      # should show no [swarm] entries when idle
node .loop/worktree.mjs cleanup   # force-remove any dangling swarm worktrees
```

## License

MIT
