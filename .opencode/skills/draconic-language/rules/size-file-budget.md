---
title: File size budget
impact: CRITICAL
impactDescription: Huge spine files hide seams and stall review
tags: [size, layout]
---

## File size budget

Target ≤1000 lines per new `rs` file. Hard limit 1250. Past the hard limit, split before adding more.

**Incorrect:** another 2000 lines in `crates/draconic-parser/src/lib.rs` or a new feature dumped into `draconic-backend-llvm/src/lib.rs`.

**Correct:** a nested module at a real seam, copied from neighbors:

```
crates/draconic-backend-llvm/src/
  lib.rs              # dispatch only
  es_expr.rs          # is_es_expr_module + emit_es_expr
  host_tcp.rs
crates/draconic-check/src/
  lib.rs
  host_api.rs
```

**Notes.** Count the whole file. Existing spine files (parser, check, ir) already exceed the budget; grow them only with a split plan. Generated or vendored files are exempt. If a split would be a pass-through, move behaviour behind the interface instead. See `arch-deep-modules` and `llvm-adapter-dispatch`.
