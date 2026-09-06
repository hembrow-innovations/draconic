---
title: Deep modules at crate seams
impact: CRITICAL
impactDescription: Shallow pass-throughs spread change across stages
tags: [arch, module, seam, depth]
---

## Deep modules at crate seams

A **module** is anything with an interface and an implementation. Design **deep** modules: lots of behaviour behind a small interface, placed at a clean **seam**. Callers and tests cross that same seam.

Use these words: module, interface, implementation, depth, seam, adapter, leverage, locality.

**Incorrect:** a wide bag of optional flags on every stage, and a CLI that reimplements Script vs Module.

**Correct:** Frontend's six entries hide parse, link, check, and lower. LLVM adapters hide subset lowering behind `is_*_module` + `emit_*`.

**Notes.** Deletion test: if deleting the module makes complexity vanish, it was a pass-through. One adapter is a hypothetical seam; two adapters make it real. Load **codebase-design** when the interface itself is the work. See `arch-crate-seams` and `llvm-adapter-dispatch`.
