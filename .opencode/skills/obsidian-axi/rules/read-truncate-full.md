---
title: Bodies cap at 1000 characters
impact: HIGH
impactDescription: truncated reads look complete unless you take --full
tags: [read, truncate]
---

## Bodies cap at 1000 characters

`read` trims note bodies to 1000 characters and prints a size hint. AXI truncation. The hint is the signal that more exists.

**Incorrect:** Treating a trimmed body as the whole note, then rewriting from that fragment.

**Correct:** `obsidian-axi read docs/guides/guides-heio-stack.md --vault docs` for a skim. `... --full` when you need every line.

Notes: Prefer `--metadata` when you only need tags, size, or frontmatter (`read-metadata`). Agent `Read` is fine for a file already in session.
