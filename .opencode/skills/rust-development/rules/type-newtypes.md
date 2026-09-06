---
title: Newtypes for ids and positions
impact: HIGH
impactDescription: Raw u32 mixes spans with symbol ids
tags: [type, newtype]
---

## Newtypes for ids and positions

Ids and positions are tuple newtypes with a public inner value: `BytePos(pub u32)`, `ErrorCode(pub u32)`, `SymbolId(pub u32)`. `Span` and `Location` are `Copy` structs.

**Incorrect:**

```rust
pub struct Token {
    pub start: u32,
    pub symbol: u32,
}
```

**Correct:**

```rust
pub struct BytePos(pub u32);
pub struct SymbolId(pub u32);

pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}
```

**Notes.** `LocalId` aliases `SymbolId`. Keep these `Copy` + `Hash` where the neighbors are. Do not wrap every `String` in a newtype. See `own-box-ast`.
