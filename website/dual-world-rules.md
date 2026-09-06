---
title: Dual-world rules
section: reference
status: not-yet
---

# Dual-world rules

Dual worlds is the coexistence of JS values and native types in one Program, with explicit boundaries at the type and lowering level.

- JS values live on the GC heap
- Native types stay unboxed and off the heap
- Crossing is explicit at the type and lowering level
- A native-only Program is valid on LLVM only; the JS backend hard-errors
- A JS-only Program is valid on the JS backend only; LLVM hard-errors
- A portable program is accepted by both backends with equivalent observable behavior

Ownership-only and arena-only models were rejected so a full ECMAScript superset can sit next to native types. Catchable exceptions are JS-value failures a Program can handle with `try` / `catch`; they do not abort the process. Process abort is a different class and is not a JS value.

Learn chapter: [Dual worlds](dual-worlds.html). Types lookup: [types](types.html).
