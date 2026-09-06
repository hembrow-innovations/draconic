---
title: Rust host compiler
impact: MEDIUM
impactDescription: Self-hosting delays Dual worlds and LLVM for years
tags: [product, rust]
---

## Rust host compiler

The Compiler stays Rust. Self-hosting in Draconic is out of scope.

**Incorrect:** a Roadmap row whose Done bar is "compiler written in Draconic."

**Correct:** grow the Rust workspace. Programs the user writes are Draconic; the toolchain that compiles them is Rust.

**Notes.** ADR-0001. Public Learn/Reference is a Draconic-generated site (ADR-0010); that is a Program, not a self-hosted Compiler. See `tool-cargo-rust`.
