---
title: Dual worlds
section: learn
status: not-yet
---

# Dual worlds

Dual worlds is the coexistence of JS values and native types in one Program, with explicit boundaries at the type and lowering level.

JS values are heap-managed and follow JavaScript semantics. Native types are unboxed and outside the GC heap. The Runtime exists so a full ECMAScript superset can live next to those native types; ownership-only and arena-only models were rejected.

Both landings arrive here:

- [from JavaScript](from-javascript.html) reaches Dual worlds from ECMAScript
- [from systems](from-systems.html) reaches Dual worlds from native types and LLVM

From here Learn is one path:

- [modules](modules.html)
- [native types](native-types.html)
- [host I/O](host-io.html)
- [packages](packages.html)
