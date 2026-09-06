---
title: Enums over boolean modes
impact: HIGH
impactDescription: Bool pairs encode illegal states
tags: [type, enum]
---

## Enums over boolean modes

Modes are enums: `BindingKind`, `TokenKind`, `NativeType`, `Type`, `CompileTarget`. Independent lexer facts may stay flags (`escaped`, `legacy_octal`).

**Incorrect:**

```rust
pub fn check(program: Program, js: bool, native: bool) -> Result<CheckedProgram, Diagnostic>
```

**Correct:**

```rust
pub enum CompileTarget { Js, Native }

pub fn check_for_target(
    program: Program,
    target: CompileTarget,
) -> Result<CheckedProgram, Diagnostic>
```

**Notes.** `HostAvailability` is a struct of two bools with `NATIVE_ONLY` / `BOTH` consts — copy that when the pair is a closed set of combinations, not an open mode. See `host-registry`.
