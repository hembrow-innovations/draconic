---
title: native types
section: learn
status: shipped
---

# native types

A native type is a static, unboxed systems type. Shipped scalars are `i8` through `i64`, `u8` through `u64`, `f32`, `f64`, and native `bool`. Fixed structs and fixed arrays hold those scalars. Pointers are native-only. These are not JavaScript language types and not primitives in the ECMAScript sense.

Use a native type when you want a value off the GC heap. JS numbers, objects, and strings stay JS values. Native types sit outside the heap. Dual worlds is the rule that both may appear in one Program only at explicit boundaries.

When to choose which:

- Stay on JS values when you want portable ECMAScript behavior on both backends.
- Reach for a native scalar, a fixed struct, or a fixed array when you want unboxed systems data on the LLVM path.
- Reach for a pointer only when you need address-of on LLVM. That Program is native-only.

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

## i8, u8, f32, and bool

The rest of the scalar set is the same story: annotate the binding, cross with `as` when you log a JS `number`. `i16`, `u16`, `u32`, `u64`, and `f64` follow `i8`, `u8`, and `f32`. Native `bool` is not JS `boolean`. Save this as `widths.drac`. It builds today:

```drac
let console = globalThis.console;
let tiny: i8 = 1;
let byte: u8 = 2;
let single: f32 = 1.5;
let flag: bool = true;
console.log(tiny as number, byte as number, single as number, flag);
```

Parse it, typecheck it, or run it:

```
draconic parse widths.drac
draconic check widths.drac
draconic run widths.drac
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

## Fixed arrays

A fixed array is a type alias of a native scalar tuple. Length is part of the type. Index the elements. Save this as `vec.drac`. It builds today:

```drac
let console = globalThis.console;

type Vec3 = [i32, i32, i32];
let v: Vec3 = [1, 2, 3];
console.log(v[0] as number, v[1] as number, v[2] as number);
```

Parse it, typecheck it, or run it:

```
draconic parse vec.drac
draconic check vec.drac
draconic run vec.drac
```

## Pointers

Pointers (`*T`, `&x`) are native-only: valid on LLVM, a hard error on the JS backend, never silent wrong code. There is no `drac` fence here because shipped fences must build on js.

A native Program takes an address and reads it back:

```
let x: i32 = 41;
let p: *i32 = &x;
let y: i32 = *p;
```

Build that shape with `draconic build --target native`. A longer Program with every shipped scalar, a fixed struct, a fixed array, and a pointer lives in the repository as [examples/types](https://github.com/hembrow-innovations/draconic/tree/main/examples/types).

A feature or Program that is valid on exactly one backend is native-only or JS-only. The other backend must hard-error with a diagnostic. Portable programs are those both backends can accept with equivalent observable behavior.

Continue to [host I/O](host-io.html). Lookup while you write: [types](types.html).
