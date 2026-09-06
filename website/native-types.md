---
title: native types
section: learn
status: not-yet
---

# native types

A native type is a static, unboxed systems type. Shipped examples in the public story are `i32`, `i64`, and fixed structs. These are not JavaScript language types and not primitives in the ECMAScript sense.

Use a native type when you want a value off the GC heap. JS numbers, objects, and strings stay JS values. Native types sit outside the heap. Dual worlds is the rule that both may appear in one Program only at explicit boundaries.

When to choose which:

- Stay on JS values when you want portable ECMAScript behavior on both backends.
- Reach for `i32`, `i64`, or a fixed struct when you want unboxed systems data on the LLVM path.

A feature or Program that is valid on exactly one backend is native-only or JS-only. The other backend must hard-error with a diagnostic. Portable programs are those both backends can accept with equivalent observable behavior.

Continue to [host I/O](host-io.html). Lookup while you write: [types](types.html).
