---
title: Heading delete removes the subtree
impact: HIGH
impactDescription: --op delete is not "clear the bullets"
tags: [patch, delete]
---

## Heading delete removes the subtree

`--op delete` on a heading removes the heading and everything under it, including nested subsections.

**Incorrect:** `--target-type heading --target Tasks --op delete` when you meant to empty the list under Tasks.

**Correct:** `--target-type heading --target Tasks --op replace --content ""` to clear that section's own content, or target a child with `::` (`patch-heading-scope`).

Notes: Block delete: `--target-type block --target abc123 --op delete`. Prefer `rm` only when the whole note should go (`rm-trash`).
