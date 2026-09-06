---
title: Discover the spec tree
impact: CRITICAL
tags: [discover]
---

# Discover the spec tree

Do this before reading or writing a spec in any project.

1. Read `AGENTS.md` (and `WORKSPACE.md` if it exists) for a named docs or spec layout. That file wins.
2. Search for stable filenames: `purpose.md`, `contract.md`, `test.md`. Skip scribble, scratch, and tracker trees.
3. Default home when nothing else is named: `docs/specs/<domain>/<area>/`, optional `<feature>/`.
4. Resolve `domain` and `area` from the path or from frontmatter. They should match.
5. If no spec tree exists yet, create the first folder under that default. Do not invent a second layout.

Ignore `docs/99_scribble/` and any `planning/` or tracker tree (`.heio/`, `.scratch/`, GitHub Issues). Those are not specs.

A leftover `web/{requirements,design,tasks}.md` triad is input, not the store. Promote still-true intent into purpose and contracts. Then stop touching the triad.

Done when you can point at the area folder, or you know the tree does not exist yet.
