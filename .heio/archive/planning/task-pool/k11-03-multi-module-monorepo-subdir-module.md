---
id: "k11-03-multi-module-monorepo-subdir-module"
title: "K11.03 Multi-module monorepo (subdir module paths)"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:40:40Z"
updated_at: "2026-09-03T05:16:34Z"
---

# K11.03 Multi-module monorepo (subdir module paths)

## Done

ROADMAP K11.03 is implemented test-first on the compiler target: a Program can depend on a module whose path maps to a subdirectory of a git repo; resolve, fetch, and import honor that subdir as the package root and do not treat sibling modules as the same package; `draconic-pkg` subdir tests are green and K11.03 is `done`.

## Context

Roadmap ID **K11.03** (Multi-module monorepo (subdir module paths)). Post-v1 packaging on the compiler target (ADR-0009): one git repo can host more than one module; a module path may resolve to a subdirectory of the checkout, not only the repository root. Tests in `crates/draconic-pkg` lock parse and apply. Mark K11.03 `done` only when those tests are green. Not K11 umbrella, K11.01 private git auth, K11.02 `replace`, K11.04 proxy/mirror, K11.05 yank, K03 cache layout at repo root, K06 single-module checkout, or npm/crates.io workspaces as the primary shape.

## Verify

`cargo test -p draconic-pkg subdir` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K11.03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.03), `crates/draconic-pkg`, `crates/draconic-pkg/src/lib.rs`, `crates/draconic-pkg/src/cache.rs`

## Links

[[s-k11-03]] [[ticket-61-k11-03-multi-module-monorepo-subdir-module]]
