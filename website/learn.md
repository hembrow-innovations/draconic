---
title: Learn
section: learn
status: shipped
---

# Learn

JavaScript you already know. Native types when you need them. One language, two backends.

Learn is the public path for people who already write JavaScript, TypeScript, or systems code. It is not a beginner programming course.

Draconic is a full ECMAScript superset with TypeScript-inspired static types and unboxed native systems types. The same Program compiles to JavaScript or to a native binary via LLVM. The language is early (v0.1): parse, typecheck, build, and test exist today; completeness is still growing.

Start at [Install](install.html). Get a `draconic` binary, parse a Program, and build it. Then pick a landing:

- [from JavaScript](from-javascript.html) if you already think in ECMAScript
- [from systems](from-systems.html) if you already think in Rust, Go, or C

Those landings join at [Dual worlds](dual-worlds.html): JS values and native types in one Program, at explicit boundaries. After that the path is one sequence: [modules](modules.html), [native types](native-types.html), [host I/O](host-io.html), [packages](packages.html).

Each page is tagged shipped or not-yet. Copy-paste samples appear only on shipped pages, and those samples must build. Keep [Reference](reference.html) open while you write: CLI, types, Dual-world rules, host I/O, and packages.
