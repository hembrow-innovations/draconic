---
title: Mutations are safe to retry
impact: HIGH
impactDescription: a second run is a no-op, not a duplicate
tags: [mut, idempotent]
---

## Mutations are safe to retry

Append of text the note already ends with, delete of a missing note, and move onto itself do nothing and exit 0.

**Incorrect:** Guarding retries with extra `read` round trips “to see if it already landed”.

**Correct:** Run the mutation again. Trust exit 0 plus the TOON. Duplicate bullets mean you appended different text, not that retry is unsafe.

Notes: `write` replaces. It is not the same as append. Prefer `patch` / `append` on existing notes (`mut-prefer-patch`).
