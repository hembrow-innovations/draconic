---
title: Stable diagnostic codes
impact: CRITICAL
impactDescription: Reused codes break Conformance error_code assertions
tags: [diag, codes]
---

## Stable diagnostic codes

Construct diagnostics with a message, span, and a stable code when the class already has one. Do not reuse a code for a different class.

**Incorrect:** `codes::NOT_ASSIGNABLE` on an unsupported host API.

**Correct:**

```rust
Diagnostic::new("message", span)
    .with_code(codes::HOST_API_UNSUPPORTED)
    .with_help("suggestion")
```

- **E0300–E0307**: assignable, callable, constructable, arity, return, excess/unknown property, extern type
- **E0400**: host API unsupported
- **E0401**: extern unsupported
- **E0402**: missing dynamic lib

**Notes.** Parse errors often have message + span only. Display is `message at start..end` or `[E0400] …`. Pretty print: `d.pretty(&SourceFile::new(name, src))`.
