---
title: native types
section: learn
status: not-yet
---

# native types

A native type is a static, unboxed systems type. Examples include `i32`, `i64`, and fixed structs. These are not JavaScript language types and not "primitives" in the ECMAScript sense.

Native types sit outside the GC heap. JS values stay on the heap. Dual worlds is the rule that both may appear in one Program only at explicit boundaries.

A feature or Program that is valid on exactly one backend is native-only or JS-only. The other backend must hard-error with a diagnostic. Portable programs are those both backends can accept with equivalent observable behavior.

Continue to [host I/O](host-io.html).
