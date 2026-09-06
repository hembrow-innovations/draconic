---
title: One crate per pipeline stage
impact: CRITICAL
impactDescription: Duplicate crates hide the real seam
tags: [crate, pipeline, layout]
---

## One crate per pipeline stage

The workspace already owns the pipeline. Put new behaviour in the crate that owns that seam. Copy a neighboring module in that crate.

- **draconic-diagnostics** — spans, codes, pretty diagnostics
- **draconic-lexer** / **ast** / **parser** / **linker** / **check** / **ir**
- **draconic-frontend** — Script vs Module, then check → lower
- **draconic-backend-js** / **draconic-backend-llvm**
- **draconic-runtime** / **draconic-embed**
- **draconic-pkg** / **draconic-lsp** / **draconic-cli**

**Incorrect:** a new `draconic-compile` crate that re-assembles parser + check + ir, or a second diagnostics type in the CLI.

**Correct:** extend `draconic-frontend` for compile policy; extend `draconic-check` for types; extend `draconic-pkg` for lockfiles.

**Notes.** Test packages are `tests/conformance`, `tests/integration`, `tests/packages`, `tests/test262`. Do not add a workspace member for fuzz; keep `crates/draconic-parser/fuzz` excluded. See `crate-frontend-entry`.
