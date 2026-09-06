---
title: db reset applies migrations and seed
impact: HIGH
impactDescription: new SQL files do nothing until reset
tags: [local, migrations]
---

## db reset applies migrations and seed

`supabase db reset` recreates the local database, applies every file in `supabase/migrations`, then runs seed.

**Incorrect:** Editing a migration and expecting the running database to pick it up.

**Correct:** After a new or changed **unapplied** migration, run `supabase db reset` or the project's reset script.

Notes: Do not edit already-applied files. `mig-no-edit-applied`.
