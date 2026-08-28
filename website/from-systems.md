---
title: from systems
section: learn
status: not-yet
---

# from systems

This landing assumes you already write Rust, Go, or C. Learn will not teach systems programming from scratch.

Draconic Programs can carry native types: static, unboxed systems types such as `i32` and `i64`, not JavaScript primitives. Those values stay outside the GC heap. JS values (objects, closures, strings) live in the Runtime with tracing GC so ECMAScript semantics still hold.

The LLVM backend lowers a Program to a native binary linked with that Runtime. The JS backend must hard-error native-only features rather than emit silent wrong code.

Host I/O is sockets-first on native, then thin HTTP on those sockets. Packages are git-backed, not an npm registry.

The designed join with the JavaScript landing is [Dual worlds](dual-worlds.html): both worlds in one Program, with explicit boundaries. After Dual worlds the rest of Learn is one path.
