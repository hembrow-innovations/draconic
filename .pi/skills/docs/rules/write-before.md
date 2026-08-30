---
title: Search first, then place
impact: CRITICAL
tags: [write]
---

# Search first, then place

Do this before any new note.

1. Search `docs/` for the same decision, spec, or guide. Skip `99_scribble/`.
2. If a near-dupe exists, update that file. Do not start a second one.
3. Pick the kind. See `template-kinds`.
4. Copy `templates/<kind>.md` into the destination the template names.
5. Fill required frontmatter from `templates/required-fields.md`.
6. Set `id` to the filename stem unless the kind names a different `id` rule. Set the h1 to the same string as `title`.

Create the parent directory on first write. Do not scaffold empty folders.

Ignore `docs/99_scribble/`. It is scratch. Never a source of truth.

If the note is an issue, plan, task, journal day, or working report, stop. Load the **management** skill.
