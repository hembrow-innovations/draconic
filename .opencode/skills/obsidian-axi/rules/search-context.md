---
title: Surrounding lines with --context
impact: HIGH
impactDescription: match-only lines hide whether the hit is the right note
tags: [search, context]
---

## Surrounding lines with --context

`--context N` prints N lines around each hit. Use it when the match string is common and you need to see the heading or list it sits in.

**Incorrect:** Opening every hit with `read --full` to learn why it matched.

**Correct:** `obsidian-axi search TODO --tag project --context 2 --vault docs`

Notes: Keep N small. Two lines is enough for a heading plus the match. Regex searches still take `--context` (`search-regex-path`).
