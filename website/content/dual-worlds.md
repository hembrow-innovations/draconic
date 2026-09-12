---
title: Dual worlds
section: learn
status: shipped
---

# Dual worlds

Dual worlds is the coexistence of JS values and native types in one Program, with explicit boundaries at the type and lowering level. It is not a typed-JavaScript-only story, and it is not a separate FFI language.

Two heaps of meaning:

- JS values are heap-managed and follow JavaScript semantics: objects, arrays, strings, closures, cycles. The Runtime uses tracing GC so those semantics still hold in a native binary.
- Native types are unboxed and outside the GC heap. Examples include `i32`, `i64`, and fixed structs. They are not ECMAScript primitives.

A portable program may use both in one file. Crossing a JS `number` and a native numeric type is an explicit `as`. There is no silent coercion. Save this as `boundary.drac`. It builds today:

```drac
let console = globalThis.console;
let jsCount: number = 41;
let nativeCount: i32 = jsCount as i32;
let next: number = (nativeCount as number) + 1;
let label: string = "ready";
console.log(label, next);
```

Parse it, typecheck it, or run it. Default `draconic run` target is js. The JS backend polyfills portable native scalars as ordinary JavaScript values:

```
draconic parse boundary.drac
draconic check boundary.drac
draconic run boundary.drac
```

Native binaries need an LLVM toolchain on the machine:

```
draconic build --target native boundary.drac -o boundary
./boundary
```

`string as i32` is a Checker error. Pointers (`*T`, `&x`) are native-only: valid on LLVM, a hard error on the JS backend, never silent wrong code.

Why the boundary is explicit: the two worlds do not share representation. Crossing is a type-level and lowering-level fact, not an accident of the backend. Ownership-only and arena-only runtimes were rejected because they cannot host a full ECMA-262 superset without cutting semantics.

After Frontend, Programs lower to one shared IR. The JS backend and the LLVM backend both consume that IR.

- A portable program is accepted by both backends with equivalent observable behavior after documented polyfills.
- A native-only Program is valid on LLVM only; the JS backend hard-errors with a diagnostic, never silent wrong code.
- A JS-only Program is valid on the JS backend only; LLVM hard-errors.

Both landings arrive here:

- [from JavaScript](from-javascript.html) reaches Dual worlds from ECMAScript
- [from systems](from-systems.html) reaches Dual worlds from native types and LLVM

From here Learn is one path:

- [modules](modules.html)
- [native types](native-types.html)
- [host I/O](host-io.html)
- [packages](packages.html)

Working lookup while you write: [Dual-world rules](dual-world-rules.html).
