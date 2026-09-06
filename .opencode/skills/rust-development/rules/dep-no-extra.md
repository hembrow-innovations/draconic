---
title: Do not add foreign toolchain crates
impact: MEDIUM
impactDescription: clap/anyhow/inkwell fork the stack
tags: [dep, crate]
---

## Do not add foreign toolchain crates

This tree has no clap, anyhow, tracing, inkwell, llvm-sys, or tower-lsp. Keep it that way. CLI is hand-parsed. LSP is an analysis snapshot library, not a tower-lsp server.

**Incorrect:**

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
anyhow = "1"
tower-lsp = "0.20"
```

**Correct:** extend `draconic-cli` flags by hand; extend `draconic-lsp::Analysis`; return `Diagnostic`.

**Notes.** New shared deps go through `[workspace.dependencies]` first (`tool-workspace-deps`). Lexer may keep crate-local `regress` / `unicode-id-start`. See `dep-pinned-unused`.
