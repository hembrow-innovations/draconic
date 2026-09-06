---
title: File size budget
impact: CRITICAL
impactDescription: Huge files hide seams and stall review
tags: [size, layout]
---

## File size budget

Target ≤1000 lines per `.rs` file. Soft limit from `AGENTS.md`. Past that, split by **feature seam** before adding more.

**Incorrect:** growing `draconic-check/src/lib.rs` (~9k) or `draconic-parser/src/lib.rs` (~10k) with the next host API or statement form.

**Correct:** new `es_*` / `host_*` / pkg files, as LLVM and pkg already do:

```
crates/draconic-backend-llvm/src/
  lib.rs
  es_expr.rs
  es_classes.rs
  host_fs.rs
  host_tcp.rs

crates/draconic-pkg/src/
  lib.rs
  lock.rs
  cache.rs
  get.rs
```

**Notes.** Count the whole file. Generated C (`draconic_rt.c`) is exempt. If a split would be a shallow pass-through, move behaviour behind the interface instead. Prefer not to grow the already-huge `lib.rs` files. See `arch-file-modules` and `arch-deep-modules`.
