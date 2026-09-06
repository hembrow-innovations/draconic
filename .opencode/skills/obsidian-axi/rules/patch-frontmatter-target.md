---
title: Frontmatter value lives in --target
impact: HIGH
impactDescription: --content is ignored for frontmatter patches
tags: [patch, frontmatter]
---

## Frontmatter value lives in --target

Frontmatter patches take `k=v` in `--target`, not in `--content`.

**Incorrect:** `--target-type frontmatter --target status --content done`

**Correct:** `--target-type frontmatter --target status=done --op replace`

Notes: Required docs fields still follow **docs** templates. Do not invent keys. `replace` updates the field. This is not a Dataview query (`layer-unsupported`).
