# Draconic

Draconic is a programming language: a **full ECMAScript superset** with TypeScript-inspired static types and **native systems types**, compiling to **JavaScript** and to **native binaries** (LLVM).

This monorepo is the Draconic toolchain (Rust) plus the agent **Loop** that grows the language to completion.

## Status

Bootstrap in progress. See [`ROADMAP.md`](./ROADMAP.md).

## Build

```bash
cargo build -p draconic-cli
cargo test --workspace
```

## CLI

```bash
cargo run -p draconic-cli -- parse path/to/file.dr
```

## Layout

| Path | Role |
|------|------|
| `crates/draconic-lexer` | Lexer |
| `crates/draconic-parser` | Parser |
| `crates/draconic-ast` | AST + dump |
| `crates/draconic-check` | Binder + Checker |
| `crates/draconic-ir` | Shared IR |
| `crates/draconic-backend-js` | JS backend |
| `crates/draconic-backend-llvm` | LLVM backend |
| `crates/draconic-runtime` | Native Runtime (GC, async) |
| `crates/draconic-embed` | Embed (eval-at-runtime) |
| `crates/draconic-cli` | `draconic` CLI |
| `tests/conformance` | Conformance suite |
| `ROADMAP.md` | Feature checklist (Loop source of truth) |
| `CONTEXT.md` | Domain glossary |
| `docs/adr/` | Architecture decisions |
| `.agents/skills/draconic-loop/` | Mega-loop skill |

## Agent loop

Invoke the **draconic-loop** skill to claim the next Roadmap item and implement it test-first (one item per session by default).

Or run N iterations unattended (same pattern as life-engine):

```bash
# OpenCode — default prompt = one draconic-loop each iteration
node .loop/opencode-loop.mjs 10

# Optional sleep between loops (seconds)
SLEEP=30 node .loop/opencode-loop.mjs 10

# Custom prompt / model flags after --
node .loop/opencode-loop.mjs 5 "run the draconic-loop skill once" -- -m xai/grok-4.5

# Claude Code
node .loop/claude-loop.mjs 10
```

## License

MIT
