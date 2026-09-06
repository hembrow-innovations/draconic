---
title: Worlds meet at as
impact: CRITICAL
impactDescription: Implicit assignability across Dual worlds is a silent box
tags: [dual, as, types]
---

## Worlds meet at as

JS values and native types coexist. Implicit assignability does not cross Dual worlds. The explicit boundary is `expr as T`.

**Incorrect:**

```
let n: number = 1;
let x: i32 = n;
```

**Correct:**

```
let n: number = 1;
let x: i32 = n as i32;
```

**Notes.** Allowed: JS `number` ↔ unboxed native numeric (not `bool`). Rejected with `cannot convert type … across dual-worlds boundary`: `string as i32`, `i32 as string`. Numeric literals may contextually type as native scalars. Fixtures: `tests/conformance/fixtures/types/dual/`. See `dual-no-silent-widen` and `ir-as-erased`.
