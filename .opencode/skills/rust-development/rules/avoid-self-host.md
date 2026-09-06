---
title: Compiler stays Rust
impact: LOW
impactDescription: Self-hosting reopens a multi-year bootstrap
tags: [avoid, host, rust]
---

## Compiler stays Rust

The Compiler is written in Rust permanently. Self-hosting in Draconic is out of scope. Node may run emitted JS; it is not the host compiler.

**Incorrect:** a `compiler.drac` that parses Draconic, or a Python driver that owns lowering.

**Correct:** extend the Rust crates. Ship the `draconic` CLI from `draconic-cli`.

**Notes.** ADR-0001. See `tool-cargo-edition`.
