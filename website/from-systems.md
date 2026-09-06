---
title: from systems
section: learn
status: not-yet
---

# from systems

This landing assumes you already write Rust, Go, or C. Learn will not teach systems programming from scratch.

Map what you already know:

- Native types such as `i32` and `i64` are static unboxed systems types, not JavaScript primitives. They stay outside the GC heap.
- JS values (objects, closures, strings, arrays) live in a Runtime with tracing GC so ECMAScript semantics still hold. Ownership-only and arena-only models were rejected because they cannot host a full ECMA-262 superset.
- The LLVM backend lowers a Program to a native binary linked with that Runtime. The JS backend must hard-error native-only features rather than emit silent wrong code.
- Host I/O is sockets-first on native (TCP listen, accept, connect, read, write), then thin HTTP/1.1 on those sockets. Not a Node-shaped http module as the only entry.
- Packages are git-backed, not an npm registry. Imports use a Go-like module path.

A Program that uses only JavaScript values can still be a portable program: both backends accept it. A native-only Program is valid on LLVM; the JS backend hard-errors. A JS-only Program is valid on the JS backend; LLVM hard-errors.

The designed join with the JavaScript landing is [Dual worlds](dual-worlds.html): both worlds in one Program, with explicit boundaries. After Dual worlds the rest of Learn is one path.
