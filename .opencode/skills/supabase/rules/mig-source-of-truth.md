---
title: Migrations are the schema
impact: HIGH
impactDescription: dashboard and ORM schemas drift from git
tags: [mig]
---

## Migrations are the schema

Git history of `supabase/migrations` is what staging and production will run. Studio, a second ORM, and verbal schema are not.

**Incorrect:** Adding Prisma or Drizzle models as the source of truth next to an existing migration folder.

**Correct:** Put new tables, policies, and functions in a migration. Keep any ORM as a consumer of that schema, or do not add one.

Notes: If the repo already uses `supabase/schemas` declarative files, see `mig-declarative-optional`.
