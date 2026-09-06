---
title: Purpose, then contract, then test.md
impact: HIGH
tags: [write]
---

# Purpose, then contract, then test.md

Search first. Update in place over near-dupes. Then:

1. Place the folder. See `folder-shape`.
2. Write `purpose.md` (job and fences). Copy **docs** `templates/purpose.md`.
3. Write or edit `contract.md` promises. Copy **docs** `templates/contract.md`. See `contract-anatomy`.
4. Write `test.md` mapping the tests that cover those promises. Copy **docs** `templates/test.md`.
5. Point Authority and `index.md` at the new notes.

Fill frontmatter from **docs** `templates/required-fields.md`. Set `id` to the filename stem. Set the h1 to the same string as `title`. Create the parent directory on first write. Do not scaffold empty folders.

If **docs** is installed, also follow its write-before, layout, and wikilink rules. Link notes with `[[wikilinks]]`.

New behaviour is contract-first: promise → red test (**tdd**) → green impl → harden with support tests → gate. Then update `test.md` so the map matches the suite.

Done when the folder has purpose, every behaviour unit has contract + test.md, and the hub links them.
