---
title: Frontmatter, links, tags, and vault plugins
impact: CRITICAL
tags: [obsidian]
---

# Frontmatter, links, tags, and vault plugins

Treat `docs/` as an Obsidian vault when `docs/.obsidian/` exists. Follow these conventions either way.

Every file starts with YAML. Required fields are in `templates/required-fields.md`. The h1 must equal `title`.

Docs carry `domain:`. That is the subject domain, not a folder.

Link notes with `[[note-name]]` or `[[note-name|alias]]`. Do not use relative `.md` paths.

Use frontmatter `tags: [...]`. Flavor rides on tags, not on extra status values.

Checkboxes use the Tasks format when the plugin is present. `- [ ] do thing 📅 2026-07-01`.

Prefer a Dataview block over a hand-maintained index when listing notes of one kind.
