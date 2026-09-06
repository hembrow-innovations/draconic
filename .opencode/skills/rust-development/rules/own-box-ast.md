---
title: Box recursive AST and IR
impact: HIGH
impactDescription: Rc in the compiler graph fights mutation and lowering
tags: [own, ast, box]
---

## Box recursive AST and IR

AST and IR use `#[derive(Debug, Clone, PartialEq)]` and `Box<Stmt>` / `Box<Expr>` for recursive nodes. Not `Rc`. `Span`, `BytePos`, `Type`, and `NativeType` are `Copy`. `JsString` owns `Vec<u16>`.

**Incorrect:**

```rust
pub enum Expr {
    Binary { left: Rc<Expr>, right: Rc<Expr>, op: BinaryOp },
}
```

**Correct:**

```rust
pub enum Expr {
    Binary { left: Box<Expr>, right: Box<Expr>, op: BinaryOp, span: Span },
}
```

**Notes.** `Rc<RefCell<…>>` belongs to LLVM JS-value interpreters, not the compiler IR. `Arc` is rare (linker `PackageLinkContext`). See `own-no-global` and `avoid-cow`.
