---
title: Do not shrink the workspace oracle
impact: LOW
impactDescription: --lib --bins mints false greens
tags: [avoid, test, oracle]
---

## Do not shrink the workspace oracle

When the promise is `cargo test --workspace`, keep that command. Narrow CHECKs only when the Done bar is actually a smaller surface.

**Incorrect:** rewriting a failing workspace CHECK to `--lib --bins` plus one package test, then marking the slice met.

**Correct:** keep `cargo test --workspace`. If the budget misses with `match=yes`, that is ADR-0012, not a product hang.

**Notes.** See `test-workspace`.
