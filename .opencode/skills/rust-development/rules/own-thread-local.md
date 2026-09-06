---
title: thread_local only at known seams
impact: MEDIUM
impactDescription: New thread-locals hide reentrancy
tags: [own, thread-local]
---

## thread_local only at known seams

`thread_local! { RefCell<…> }` exists for linker `PackageLinkContext` and LLVM JS-value interp (`this` / `new.target`). Do not add another for a new compiler stage.

**Incorrect:** a thread-local intern pool inside `draconic-check`.

**Correct:** pass context on the stack (`LowerCtx`, `&mut Checker`). Copy the linker/LLVM pattern only when the same reentrancy exists.

**Notes.** See `own-no-global`. Prefer explicit parameters; they are the test surface.
