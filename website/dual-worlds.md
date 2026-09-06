---
title: Dual worlds
section: learn
status: not-yet
---

# Dual worlds

Dual worlds is the coexistence of JS values and native types in one Program, with explicit boundaries at the type and lowering level. It is not a typed-JavaScript-only story, and it is not a separate FFI language.

Two heaps of meaning:

- JS values are heap-managed and follow JavaScript semantics: objects, arrays, strings, closures, cycles. The Runtime uses tracing GC so those semantics still hold in a native binary.
- Native types are unboxed and outside the GC heap. Examples include `i32`, `i64`, and fixed structs. They are not ECMAScript primitives.

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
