---
title: types
section: reference
status: shipped
---

# types

Types are TypeScript-inspired. The Checker is not tsc: it does not compile existing TypeScript projects, and it does not match tsc flag-for-flag. The JS backend emits JavaScript, not TypeScript. Drop-in TypeScript migration is out of scope so native types and Dual worlds are not constrained by tsc's type-erasure model.

Two universes in one Program:

- JS values are heap-managed and follow JavaScript semantics.
- A native type is a static unboxed systems type such as `i32`, `i64`, `i8`, `u8`, `f32`, native `bool`, a fixed struct, or a fixed array, outside the GC heap. Do not call it a primitive. Pointers are native-only.

Lookup while you write. Open [from JavaScript](from-javascript.html) or [native types](native-types.html) when you want the lesson instead.

## JS values

Annotate JS values when you want the Checker to check them. Save this as `greet.drac`. It builds today:

```drac
let console = globalThis.console;

function greet(name: string): string {
  return "hello " + name;
}

console.log(greet("from Draconic"));
```

```
draconic parse greet.drac
draconic check greet.drac
draconic run greet.drac
```

Unannotated JS stays permissive. A free `console` is unresolved. Bind the host object from `globalThis.console` before you log.

## Object types

Object annotations and `type` aliases are structural shapes. A matching literal assigns; a missing or extra property is a Checker error. Save this as `user.drac`. It builds today:

```drac
let console = globalThis.console;

type User = { name: string; id: number };
let user: User = { name: "Ada", id: 1 };
let named: { name: string } = user;

function label(row: { name: string }): string {
  return row.name;
}

console.log(label(user), named.name);
```

```
draconic parse user.drac
draconic check user.drac
draconic run user.drac
```

There is no `interface`, no optional `?` fields, and no index signatures. Untyped objects stay dynamic.

## Unions and intersections

`A | B` accepts each member. `A & B` merges object shapes. Unions narrow on `typeof`. That is the promised narrowing, not TypeScript's full control-flow graph. Save this as `tag.drac`. It builds today:

```drac
let console = globalThis.console;

type StrOrNum = string | number;
type Named = { name: string } & { id: number };

function tag(x: string | number): string {
  if (typeof x === "string") {
    return x;
  }
  return "num";
}

let row: Named = { name: "Ada", id: 1 };
console.log(tag("hi"), tag(1), row.name);
```

```
draconic parse tag.drac
draconic check tag.drac
draconic run tag.drac
```

## Generics

Generic aliases and generic functions are Checker features. Call sites infer type arguments. Arity and argument mismatches reject. Save this as `box.drac`. It builds today:

```drac
let console = globalThis.console;

type Box<T> = { value: T };

function id<T>(x: T): T {
  return x;
}

function box<T>(v: T): Box<T> {
  return { value: v };
}

console.log(id("hi"), box(42).value);
```

```
draconic parse box.drac
draconic check box.drac
draconic run box.drac
```

## Native types

Numeric literals may contextually type as `i32` or `i64`. Crossing back to a JS `number` is an explicit `as`. Save this as `width.drac`. It builds today:

```drac
let console = globalThis.console;
let count: i32 = 41;
let wide: i64 = 42;
console.log(count as number, wide as number);
```

```
draconic parse width.drac
draconic check width.drac
draconic run width.drac
```

## i8, u8, f32, and bool

`i16`, `u16`, `u32`, `u64`, and `f64` follow the same scalar rule. Native `bool` is not JS `boolean`. Save this as `widths.drac`. It builds today:

```drac
let console = globalThis.console;
let tiny: i8 = 1;
let byte: u8 = 2;
let single: f32 = 1.5;
let flag: bool = true;
console.log(tiny as number, byte as number, single as number, flag);
```

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

```
draconic parse point.drac
draconic check point.drac
draconic run point.drac
```

## Fixed arrays

A fixed array is a type alias of a native scalar tuple. Length is part of the type. Save this as `vec.drac`. It builds today:

```drac
let console = globalThis.console;

type Vec3 = [i32, i32, i32];
let v: Vec3 = [1, 2, 3];
console.log(v[0] as number, v[1] as number, v[2] as number);
```

```
draconic parse vec.drac
draconic check vec.drac
draconic run vec.drac
```

## Pointers

Pointers (`*T`, `&x`) are native-only: valid on LLVM, a hard error on the JS backend, never silent wrong code. There is no `drac` fence here because shipped fences must build on js.

```
let x: i32 = 41;
let p: *i32 = &x;
let y: i32 = *p;
```

Build that shape with `draconic build --target native`. The in-repo Program is [examples/types](https://github.com/hembrow-innovations/draconic/tree/main/examples/types).

## Crossing

Crossing a JS `number` and a native numeric type is an explicit `as`. There is no silent coercion. Save this as `boundary.drac`. It builds today:

```drac
let console = globalThis.console;
let jsCount: number = 41;
let nativeCount: i32 = jsCount as i32;
let next: number = (nativeCount as number) + 1;
console.log(next);
```

```
draconic parse boundary.drac
draconic check boundary.drac
draconic run boundary.drac
```

`string as i32` is a Checker error. Pointers (`*T`, `&x`) are native-only: valid on LLVM, a hard error on the JS backend, never silent wrong code.

## Backend fit

- A portable program is accepted by both backends with equivalent observable behavior after documented polyfills.
- Native-only features must hard-error on the JS backend, never silent wrong code.
- JS-only features must hard-error on the LLVM backend.

Dual-world boundaries are listed under [Dual-world rules](dual-world-rules.html). Learn path: [from JavaScript](from-javascript.html), then [Dual worlds](dual-worlds.html), then [native types](native-types.html).
