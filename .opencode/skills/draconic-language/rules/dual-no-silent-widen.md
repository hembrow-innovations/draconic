---
title: No silent native widen
impact: CRITICAL
impactDescription: i32 as i64 would invent a conversion the IR does not own
tags: [dual, as, native]
---

## No silent native widen

Native widths do not convert to each other with `as`. Hop through `number` when a conversion is required.

**Incorrect:**

```
let a: i32 = 1;
let b: i64 = a as i64;
```

**Correct:**

```
let a: i32 = 1;
let b: i64 = (a as number) as i64;
```

**Notes.** `null` → `*T` and JS `boolean` → native `bool` are the listed exceptions, not a general conversion lattice. See `dual-as-boundary`.
