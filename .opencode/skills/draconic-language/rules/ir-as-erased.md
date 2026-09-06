---
title: as is type-only
impact: HIGH
impactDescription: A runtime coerce node for every as duplicates the Checker
tags: [ir, as, dual]
---

## as is type-only

IR erases `as`. Types on IR nodes already reflect the Dual-worlds boundary. LLVM inserts numeric casts when the lowered types differ.

**Incorrect:** an IR `Expr::Cast` for every `n as i32`.

**Correct:** Checker validates `as`; `lower` drops the node; JS treats native scalars as numbers; LLVM `native_ints` emits int↔float casts when needed.

**Notes.** See `dual-as-boundary`. Do not invent a second conversion pass in the JS backend.
