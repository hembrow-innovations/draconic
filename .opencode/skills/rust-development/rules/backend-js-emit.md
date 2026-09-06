---
title: JS backend emits ECMAScript text
impact: HIGH
impactDescription: Emitting TypeScript or skipping IR forks ADR-0005
tags: [backend, js]
---

## JS backend emits ECMAScript text

`emit_js(&Module) -> Result<String, Diagnostic>` lowers shared IR to ECMAScript source. Optional source maps via `emit_js_with_map`. JS polyfill strings come from `draconic-runtime`.

**Incorrect:**

```rust
pub fn emit_ts(program: &Program) -> String { /* tsc-like */ }
```

**Correct:**

```rust
pub fn emit_js(module: &Module) -> Result<String, Diagnostic> {
    Ok(emit_js_full(module, None)?.code)
}
```

**Notes.** ADR-0005: TS-inspired checker, JS backend emits JS, not TS. Native policy is in this crate (`dual-js-policy`). Do not pull Node into the crate. See `ir-shared`.
