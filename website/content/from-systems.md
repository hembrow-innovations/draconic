---
title: from systems
section: learn
status: shipped
---

# from systems

This landing assumes you already write Rust, Go, or C. Learn will not teach systems programming from scratch.

A Program that looks like JavaScript is still a Program, including on this landing. Save this as `hello.drac`. It builds today:

```drac
let console = globalThis.console;
console.log("hello from Draconic");
```

A free `console` is unresolved. Bind the host object from `globalThis.console` before you log.

Parse it, emit JavaScript, or let the toolchain run it. Default `draconic run` target is js:

```
draconic parse hello.drac
draconic build --target js hello.drac -o hello.js
draconic run hello.drac
```

Native binaries need an LLVM toolchain on the machine:

```
draconic build --target native hello.drac -o hello
./hello
```

Native types such as `i32` and `i64` are static unboxed systems types, not JavaScript primitives. They stay outside the GC heap. Numeric literals may contextually type as those scalars. Save this as `add.drac`. It builds today:

```drac
let console = globalThis.console;

function add(x: i32, y: i32): i32 {
  return x + y;
}

let total: i32 = add(20, 22);
console.log(total as number);
```

Parse it, typecheck it, or run it:

```
draconic parse add.drac
draconic check add.drac
draconic run add.drac
```

`total as number` is an explicit Dual-worlds boundary. There is no silent coercion. JS values (objects, closures, strings, arrays) live in a Runtime with tracing GC so ECMAScript semantics still hold. Ownership-only and arena-only models were rejected because they cannot host a full ECMA-262 superset.

The LLVM backend lowers a Program to a native binary linked with that Runtime. The JS backend polyfills portable native scalars as ordinary JavaScript values. Pointers (`*T`, `&x`) are native-only: valid on LLVM, a hard error on the JS backend, never silent wrong code.

A Program that uses only JavaScript values can still be a portable program: both backends accept it. A native-only Program is valid on LLVM; the JS backend hard-errors. A JS-only Program is valid on the JS backend; LLVM hard-errors.

A longer native Program with TCP listen and HTTP/1.1 lives in the repository as [HTTP echo](https://github.com/hembrow-innovations/draconic/tree/main/examples/http-echo).

Host I/O is sockets-first on native, then thin HTTP/1.1 on those sockets. Packages are git-backed, not an npm registry.

The designed join with the JavaScript landing is [Dual worlds](dual-worlds.html): both worlds in one Program, with explicit boundaries. After Dual worlds the rest of Learn is one path.
