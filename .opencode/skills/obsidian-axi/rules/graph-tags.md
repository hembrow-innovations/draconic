---
title: Tag counts roll up
impact: HIGH
impactDescription: #project includes #project/axi
tags: [graph, tags]
---

## Tag counts roll up

`tags` lists tags with counts. Nested tags roll up: `#project` includes `#project/axi`. `tags files <tag>` lists the notes.

**Incorrect:** Summing nested tags by hand, or grepping `#project` and missing `#project/axi`.

**Correct:**

```sh
obsidian-axi tags --vault docs
obsidian-axi tags files project/axi --vault docs
```

Notes: `--tag project` on `search` also matches nested (`search-narrow`). Docs notes use frontmatter `tags: [...]`, not ad-hoc `#` soup in the body as the source of truth.
