---
title: A spec is a folder
impact: HIGH
tags: [layout]
---

# A spec is a folder

Path: `docs/specs/<domain>/<area>/`. When the area has more than one coherent unit, nest `docs/specs/<domain>/<area>/<feature>/`.

`domain` in the path equals frontmatter `domain:`. `area` equals frontmatter `area:`. Feature slug is the concern (`crud`, `ui`, `members`), not a framework (`web`, `hooks`).

Skip a level when it would have a single child. No domains → `docs/specs/<area>/`. One unit → files live on the area. Create a folder when the first file needs it.

Worked shape (example, not law):

```text
docs/specs/
  index.md
  features/
    calendar/
      index.md
      purpose.md
      contract.md
      test.md
    tasks/
      index.md
      purpose.md
      crud/
        contract.md
        test.md
      lists/
        contract.md
        test.md
```

## Files

- **purpose.md** — job and fences. Copy **docs** `templates/purpose.md`. See `purpose-job`.
- **contract.md** — promises. Copy **docs** `templates/contract.md`. See `contract-anatomy`. One coherent unit per file.
- **test.md** — which tests cover this unit, how, and why. Copy **docs** `templates/test.md`. Lives beside the `contract.md` it covers.
- **index.md** — hub of wikilinks to purpose, contracts, tests, ADRs. No duplicated promises.
- **data-map.md** — only when data topology is non-obvious.

A `spec-<slug>.md` is optional extra narrative inside the folder when purpose and contract cannot hold a how/shape note (CLI surface, protocol). Copy **docs** `templates/spec.md`. It is not the living product spec.

Stable filenames are the AI index. A glob for the kind is the catalog. The path is the human taxonomy. One concern per file so a task reads 1–3 notes, not a dump.
