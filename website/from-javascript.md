---
title: from JavaScript
section: learn
status: shipped
---

# from JavaScript

This landing assumes you already write JavaScript or TypeScript. Learn will not teach ECMAScript from scratch.

Draconic is a full ECMAScript superset. A Program that looks like JavaScript is still a Program. Save this as `hello.drac`:

```drac
let console = globalThis.console;
console.log("hello from Draconic");
```

Parse it, emit JavaScript, or let the toolchain run it. Default `draconic run` target is js:

```
draconic parse hello.drac
draconic build --target js hello.drac -o hello.js
draconic run hello.drac
```

The JS backend emits JavaScript, not TypeScript. You do not get a `.ts` file out. Types are TypeScript-inspired: the surface is familiar, but the Checker is not tsc and does not aim to compile existing TypeScript projects. Drop-in migration of a TypeScript repo is out of scope.

Native types such as `i32` and `i64` are the extra, not a typed-JS-only story. You do not need them on this landing. A Program that stays in JavaScript values is a portable program: both backends can accept it with equivalent observable behavior after documented polyfills.

When you need unboxed systems types and the LLVM path, continue at [Dual worlds](dual-worlds.html). That is the join. The rest of Learn is one path from there.
