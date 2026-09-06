---
title: Heading patch stays in that section
impact: HIGH
impactDescription: nested subsections are not the same heading
tags: [patch, heading]
---

## Heading patch stays in that section

`--target-type heading` writes that heading's own content. Nested subsections stay untouched. A child heading needs `::`.

**Incorrect:** `--target Tasks` expecting to land under `### Today`.

**Correct:** `--target-type heading --target "Tasks::Today" --op prepend --content "- [ ] first"`

Notes: `--op` is `prepend`, `replace`, or `delete`. Append to a heading stays in that section, not in children. Block targets use `--target-type block --target <id>`.
