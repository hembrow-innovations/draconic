---
title: native types
section: learn
status: shipped
---

# native types

A native type is a static, unboxed systems type. Shipped examples in the public story are `i32`, `i64`, and fixed structs. These are not JavaScript language types and not primitives in the ECMAScript sense.

Use a native type when you want a value off the GC heap. JS numbers, objects, and strings stay JS values. Native types sit outside the heap. Dual worlds is the rule that both may appear in one Program only at explicit boundaries.

When to choose which:

- Stay on JS values when you want portable ECMAScript behavior on both backends.
- Reach for `i32`, `i64`, or a fixed struct when you want unboxed systems data on the LLVM path.

## i32 and i64

Numeric literals may contextually type as those scalars. Crossing back to a JS `number` is an explicit `as`. There is no silent coercion. Save this as `width.drac`. It builds today:

```drac
let console = globalThis.console;
let count: i32 = 41;
let wide: i64 = 42;
console.log(count as number, wide as number);
```

Parse it, typecheck it, or run it. Default `draconic run` target is js. The JS backend polyfills portable native scalars as ordinary JavaScript values:

```
draconic parse width.drac
draconic check width.drac
draconic run width.drac
```

Native binaries need an LLVM toolchain on the machine:

```
draconic build --target native width.drac -o width
./width
```

## Fixed structs

A fixed struct is a type alias of native scalar fields. Initialize it with an object literal and read fields. Save this as `point.drac`. It builds today:

```drac
let console = globalThis.console;

type Point = { x: i32; y: i32 };
let p: Point = { x: 10, y: 20 };
console.log(p.x as number, p.y as number);
```

Parse it, typecheck it, or run it:

```
draconic parse point.drac
draconic check point.drac
draconic run point.drac
```

Pointers (`*T`, `&x`) are native-only: valid on LLVM, a hard error on the JS backend, never silent wrong code.

A feature or Program that is valid on exactly one backend is native-only or JS-only. The other backend must hard-error with a diagnostic. Portable programs are those both backends can accept with equivalent observable behavior.

Continue to [host I/O](host-io.html). Lookup while you write: [types](types.html).
