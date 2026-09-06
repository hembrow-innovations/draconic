---
title: Unit tests live in the same file
impact: CRITICAL
impactDescription: Split test modules hide the seam
tags: [test, unit, layout]
---

## Unit tests live in the same file

Crate unit tests are `#[cfg(test)] mod tests` at the bottom of the same `.rs` file. Parser, check, IR, lexer, and frontend all do this.

**Incorrect:** a crate-level `tests/compile.rs` that reaches into private helpers, or a `src/lib_tests.rs` with no owning module.

**Correct:**

```rust
pub fn compile_source(source: &str) -> Result<Module, Diagnostic> { /* … */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_source_lowers_script() {
        let module = compile_source("1 + 2").expect("compile");
        assert!(!module.stmts.is_empty());
    }
}
```

**Notes.** Runtime may split huge tests into `host_abi_tests.rs` still declared `#[cfg(test)] mod` from `lib.rs`. Language semantics go to `tests/conformance`, not more unit tests of emit internals. Load **tdd**. See `test-conformance`.
