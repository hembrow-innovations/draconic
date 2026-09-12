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

A free `console` is unresolved. Bind the host object from `globalThis.console` before you log. That is why every copy-paste sample on this path starts that way.

Parse it, emit JavaScript, or let the toolchain run it. Default `draconic run` target is js:

```
draconic parse hello.drac
draconic build --target js hello.drac -o hello.js
draconic run hello.drac
```

The JS backend emits JavaScript, not TypeScript. You do not get a `.ts` file out. Types are TypeScript-inspired: the surface is familiar, but the Checker is not tsc and does not aim to compile existing TypeScript projects. Drop-in migration of a TypeScript repo is out of scope.

You can still write functions and annotations on JS values. Save this as `greet.drac`. It builds today:

```drac
let console = globalThis.console;

function greet(name: string): string {
  return "hello " + name;
}

console.log(greet("from Draconic"));
```

Parse it, typecheck it, or run it:

```
draconic parse greet.drac
draconic check greet.drac
draconic run greet.drac
```

A longer portable Program with `for`, `if`, and the same host console lives in the repository as [FizzBuzz](https://github.com/hembrow-innovations/draconic/tree/main/examples/fizzbuzz).

## Todo

`draconic run` executes the emitted JavaScript with Node. That is not the browser path.

`draconic build --target js` writes a JavaScript file you load from HTML. Save this as `todo.drac`. It builds today:

```drac
let doc = globalThis.document;
let storage = globalThis.localStorage;
let app = doc.getElementById("app");
storage.setItem("draconic-todo", "[]");
```

A free `document` or `localStorage` is unresolved. Bind them from `globalThis`, the same way this landing binds `console`.

Emit the file, then load it with a script tag. Do not `draconic run` this Program: Node has no DOM.

```
draconic build --target js todo.drac -o todo.js
```

```
<script src="todo.js"></script>
```

Host I/O names such as `stdoutWrite` and `tcpListen` are the machine path, not the DOM path.

Copy [Todo](https://github.com/hembrow-innovations/draconic/tree/main/examples/todo) for a longer Program: `document`, `localStorage`, and a native static host.

Native types such as `i32` and `i64` are the extra, not a typed-JS-only story. You do not need them on this landing. A Program that stays in JavaScript values is a portable program: both backends can accept it with equivalent observable behavior after documented polyfills.

When you need unboxed systems types and the LLVM path, continue at [Dual worlds](dual-worlds.html). That is the join. The rest of Learn is one path from there.
