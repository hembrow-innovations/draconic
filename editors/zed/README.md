# Draconic for Zed

Syntax highlighting for `.drac` files. Uses the TypeScript tree-sitter grammar plus Draconic native types (`i32`, `u8`, `f64`, …) and `extern`.

## Install

Zed does not ship this yet. Install it as a **dev extension**:

1. Open the Extensions view (`zed: extensions`).
2. Click **Install Dev Extension**.
3. Choose this directory — the one that contains `extension.toml`: `editors/zed`.
   Do not choose the repo root, `languages/draconic`, or `grammars/draconic`.

First install compiles the grammar (needs network). Then reopen a `.drac` file; the status bar language should read **Draconic**.
