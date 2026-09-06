---
title: Meta is a sidecar
impact: HIGH
impactDescription: JSON frontmatter in .drac forks the harness
tags: [test, meta, fixture]
---

## Meta is a sidecar

Fixture metadata is a line-oriented `.meta` file next to the `.drac`. Dependency modules are bare `.drac` with no meta.

**Incorrect:** JSON frontmatter inside the Program, or a `.meta` on every imported file.

**Correct:**

```text
id: smoke/let-add
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
```

Keys include `targets`, `js.exit` / `js.check` / `js.stdout` / `js.error` / `js.error_code`, the `native.*` twins, `args`, `stdin`, `grants`. Default without meta: js only, exit 0. `native.check` is not a key.

**Notes.** In-language `describe(` / `it(` suites can omit meta (L05.04). See `test-conformance-fixtures`.
