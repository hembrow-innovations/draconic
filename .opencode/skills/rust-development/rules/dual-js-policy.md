---
title: Native-only features fail on js
impact: CRITICAL
impactDescription: Emitting fake JS hides the missing backend
tags: [dual, js, policy]
---

## Native-only features fail on js

On the js target, native pointers, `extern "C"`, and host APIs with `HostAvailability::NATIVE_ONLY` return a diagnostic. Do not emit a JS polyfill that pretends they work.

**Incorrect:**

```rust
if target == CompileTarget::Js {
    return Ok("null".into()); // pointer
}
```

**Correct:**

```rust
.with_code(codes::HOST_API_UNSUPPORTED)
.with_help("compile with the native backend, or remove the host call")
```

N04 JS policy in `emit_js`:

- Native scalars, layout structs, fixed arrays: polyfill/erase as ordinary JS
- `*T`, `&x`, `*p = v`: hard-error
- `extern "C"` / FFI: hard-error (`F08.01`)

**Notes.** Register availability in `host_api.rs`, do not special-case names in the emitter. See `err-hard-error` and `host-registry`.
