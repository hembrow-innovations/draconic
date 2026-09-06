---
title: The dashboard is not the schema
impact: CRITICAL
impactDescription: Studio-only tables vanish on db reset
tags: [pitfall, migrations]
---

## The dashboard is not the schema

Local Studio is a scratchpad. `db reset` rebuilds from `supabase/migrations` and seed. Cloud dashboard edits are not in git.

**Incorrect:** Creating a table only in Studio, then wondering why reset deleted it.

**Correct:** Put DDL in a migration file. Reset to apply it. Use `db diff -f` if you prototyped in Studio and want the SQL captured.

Notes: `mig-source-of-truth`.
