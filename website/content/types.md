---
title: types
section: reference
status: shipped
---

# types

Types are TypeScript-inspired. The Checker is not tsc: it does not compile existing TypeScript projects, and it does not match tsc flag-for-flag. The JS backend emits JavaScript, not TypeScript. Drop-in TypeScript migration is out of scope so native types and Dual worlds are not constrained by tsc's type-erasure model.

Two universes in one Program:

- JS values are heap-managed and follow JavaScript semantics.
- A native type is a static unboxed systems type such as `i32`, `i64`, or a fixed struct, outside the GC heap. Do not call it a primitive.

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

A fixed struct is a type alias of native scalar fields. Initialize it with an object literal and read fields.

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
