---
title: Stable codes live in diagnostics::codes
impact: CRITICAL
impactDescription: Ad-hoc numbers break U09 labels and fixtures
tags: [err, codes, diagnostic]
---

## Stable codes live in diagnostics::codes

Checker and host diagnostics take codes from `draconic_diagnostics::codes`. Display is `E` plus four digits (`E0300`). Assign once; keep stable.

**Incorrect:**

```rust
Diagnostic::new("not assignable", span).with_code(ErrorCode(12))
```

**Correct:**

```rust
use draconic_diagnostics::{codes, Diagnostic, Span};

pub fn extern_unsupported_on_js_diagnostic(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        format!("extern \"C\" function `{name}` is unsupported on js target (native-only FFI)"),
        span,
    )
    .with_code(codes::EXTERN_UNSUPPORTED)
    .with_help("compile with the native backend, or remove the extern declaration")
}
```

**Notes.** Existing codes: `NOT_ASSIGNABLE` 300, `NOT_CALLABLE` 301, `HOST_API_UNSUPPORTED` 400, `EXTERN_UNSUPPORTED` 401, `MISSING_DYNAMIC_LIB` 402. Add a named `pub const` in `codes` before using a new number. Conformance may assert `error_code`. See `err-hard-error`.
