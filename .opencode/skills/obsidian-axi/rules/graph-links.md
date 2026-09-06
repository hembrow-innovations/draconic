---
title: Backlinks, broken, orphans
impact: HIGH
impactDescription: grep cannot tell a resolved wikilink from a miss
tags: [graph, links]
---

## Backlinks, broken, orphans

`links <path>` shows incoming references. `--broken` lists unresolved links. `--orphans` lists notes nothing points at.

**Incorrect:** Grepping `[[Roadmap]]` across `docs/` and calling that the backlink set.

**Correct:**

```sh
obsidian-axi links "architecture-heio-stack.md" --vault docs
obsidian-axi links --broken --vault docs
obsidian-axi links --orphans --vault docs
```

Notes: Resolution order is `graph-link-resolve`. Embeds are not broken. After a rename, run `--broken` then fix leftovers per **docs**.
