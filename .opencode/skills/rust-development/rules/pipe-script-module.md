---
title: Script vs Module is parse-driven
impact: CRITICAL
impactDescription: Substring heuristics mis-classify entries
tags: [pipe, module, script]
---

## Script vs Module is parse-driven

Frontend chooses Script parse vs Module link from the parsed program, not from a source substring. Linked entries use the Module goal (top-level `await` allowed). Scripts reject top-level `await`.

**Incorrect:**

```rust
if source.contains("import ") || source.contains("export ") {
    link_entry(entry)
} else {
    parse(source)
}
```

**Correct:**

```rust
match parse(&source) {
    Ok(program) if program_has_module_syntax(&program) => Ok((link_entry(entry)?, true)),
    Ok(program) => Ok((program, false)),
    Err(script_err) => match parse_module(&source) {
        Ok(program) if program_has_module_syntax(&program) => Ok((link_entry(entry)?, true)),
        _ => Err(script_err),
    },
}
```

**Notes.** Keep this policy in `draconic-frontend` (`load_program`). Embed and single-buffer eval use `compile_source` (Script). Need Module without a graph: `compile_source_module`. See `crate-frontend-entry`.
