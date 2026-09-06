---
title: Unsupported surfaces hard-error
impact: CRITICAL
impactDescription: Silent subsetting ships wrong JS
tags: [err, hard-error, dual]
---

## Unsupported surfaces hard-error

Native-only features on the js target, and unsupported IR on LLVM, return `Err(Diagnostic)`. Empty LLVM programs may emit the B08 hello stub; non-empty unsupported IR must not.

**Incorrect:**

```rust
pub fn emit_js(module: &Module) -> Result<String, Diagnostic> {
    if module.has_extern_ffi {
        return Ok("/* skipped extern */".into());
    }
    Ok(emit_body(module))
}
```

**Correct:**

```rust
pub fn emit_js(module: &Module) -> Result<String, Diagnostic> {
    if module.has_extern_ffi {
        return Err(extern_unsupported_on_js_diagnostic("…", Span::dummy()));
    }
    Ok(emit_js_full(module, None)?.code)
}
```

**Notes.** Pointers, `extern "C"`, and host APIs marked `NATIVE_ONLY` fail on js with `codes::EXTERN_UNSUPPORTED` / `HOST_API_UNSUPPORTED`. Prefer a hard diagnostic over wrong code. Split Roadmap rows (`js` / `native`) rather than subsetting. See `dual-js-policy`.
