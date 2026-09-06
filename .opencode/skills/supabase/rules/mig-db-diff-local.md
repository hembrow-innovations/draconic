---
title: Capture Studio prototypes with db diff
impact: MEDIUM
impactDescription: unfiled Studio tables die on reset
tags: [mig]
---

## Capture Studio prototypes with db diff

`supabase db diff -f <name>` writes a migration from changes already on the local database.

**Incorrect:** Building the whole schema in Studio and never writing a file.

**Correct:** Prototype in local Studio if you want, then `supabase db diff -f add_widgets` and read the file before you keep it. Prefer a hand-written migration when you already know the DDL.

Notes: Diff output is verbose. Trim grants you did not mean to change.
