---
title: Metadata without the body
impact: HIGH
impactDescription: pulling the body just to see tags wastes tokens
tags: [read, metadata]
---

## Metadata without the body

`read --metadata` answers tags, size, and frontmatter without the body. That is the bigger saving than `--fields`.

**Incorrect:** `obsidian-axi read architecture-heio-stack.md --full --vault docs` when you only wanted `domain:` and tags.

**Correct:** `obsidian-axi read architecture-heio-stack.md --metadata --vault docs`

Notes: Several paths work (`read-multi`). For “what links here”, use `graph-links` instead of opening bodies.
