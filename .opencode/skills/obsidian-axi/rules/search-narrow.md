---
title: Narrow search with filters
impact: HIGH
impactDescription: a bare query dumps the vault
tags: [search, filter]
---

## Narrow search with filters

`search <query>` is full text. Stack `--tag`, `--path`, `--frontmatter k=v`, and `--modified-since 7d`. Repeat `--tag` and `--frontmatter`; everything must match. `--tag project` also matches `#project/axi`.

**Incorrect:** `obsidian-axi search . --vault docs` then skimming hundreds of hits.

**Correct:** `obsidian-axi search heio --tag stack --path "guides/**" --vault docs`

Notes: `--modified-since 7d` is a duration, not a date picker. Frontmatter match is `k=v`. Add `--context` only when surrounding lines are needed (`search-context`).
