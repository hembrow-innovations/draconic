---
title: from JavaScript
section: learn
status: shipped
---

# from JavaScript

This landing assumes you already write JavaScript or TypeScript. Learn will not teach ECMAScript from scratch.

Draconic is a full ECMAScript superset. A Program that looks like JavaScript is still a Program. The JS backend emits JavaScript, not TypeScript.

Types are TypeScript-inspired: the surface is familiar, but the Checker is not tsc and does not aim to compile existing TypeScript projects. Native types are the extra, not a typed-JS-only story.

You still compile with `draconic build --target js`. When you need unboxed systems types and the LLVM path, continue at [Dual worlds](dual-worlds.html). That is the join. The rest of Learn is one path from there.
