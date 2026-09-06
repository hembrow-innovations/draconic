---
title: Commit generated types the repo already commits
impact: MEDIUM
impactDescription: uncommitted types drift from migrations
tags: [types, ci]
---

## Commit generated types the repo already commits

If the repo checks in a `database.types.ts` (or similar), regenerate it after schema changes and include it in the same work.

**Incorrect:** Changing a migration and leaving stale types, or adding a new types path the client does not import.

**Correct:** Generate to the existing path. If CI diffs types (the official environment guide does), run that check locally.

Notes: Do not invent a types file if the project does not use generated types.
