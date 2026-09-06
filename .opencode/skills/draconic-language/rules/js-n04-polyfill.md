---
title: N04 polyfill vs hard-error
impact: CRITICAL
impactDescription: Hard-erroring portable scalars, or polyfilling pointers, both lie
tags: [js, n04, native]
---

## N04 polyfill vs hard-error

On the js target, native scalars, layout structs, and fixed arrays polyfill as ordinary JS numbers/objects/arrays. Pointers, `&`/`*`, `*p =`, and `extern "C"` hard-error.

**Incorrect:** hard-error `let a: i32 = 1` on js; emit `&x` as a JS no-op.

**Correct:**

```
let a: i32 = 1;          // js: number 1
let p: *i32 = &x;        // js.error: native-only
```

Fixtures: `native/js-policy/scalar_polyfill` vs `native/js-policy/ptr_hard_error`.

**Notes.** Policy is per feature, not "all native is an error on js." See `diag-hard-error` and `dual-native-unboxed`.
