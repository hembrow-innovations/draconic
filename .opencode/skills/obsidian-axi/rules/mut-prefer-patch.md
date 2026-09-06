---
title: Patch over write
impact: HIGH
impactDescription: write replaces the whole note
tags: [mut, patch]
---

## Patch over write

`patch` changes one heading, block, or frontmatter field. `write` creates or replaces the file. Prefer patch on an existing note.

**Incorrect:** `obsidian-axi write docs/guides/guides-heio-stack.md --content "..."` to add one bullet.

**Correct:** `obsidian-axi patch docs/guides/guides-heio-stack.md --target-type heading --target Title --content "- new bullet" --vault docs`

Notes: New durable notes still go through the **docs** skill first (`layer-docs-create`). Whole-body prose on a file already in session can use agent `Edit`.
