---
title: New durable notes go through docs
impact: LOW
impactDescription: write skips kind, template, and required frontmatter
tags: [layer, docs]
---

## New durable notes go through docs

This CLI can `write` a file anywhere in the vault. This repo's durable notes need a kind, a template, a path, and required frontmatter from the **docs** skill.

**Incorrect:** `obsidian-axi write docs/guides/my-note.md --content "# My note" --vault docs`

**Correct:** Load **docs**. Copy `templates/<kind>.md`. Place per `layout-vault`. Then, if you need a heading patch later, come back here.

Notes: Search and links on existing notes stay on this CLI. `write` is for dest vaults that are not this pack's `docs/` layout, or for a file **docs** already placed.
