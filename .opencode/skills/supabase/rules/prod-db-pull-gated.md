---
title: db pull reads the hosted schema
impact: CRITICAL
impactDescription: pulls production DDL into a new migration
tags: [prod, migrations]
---

## db pull reads the hosted schema

`supabase db pull` dumps the linked remote schema into `supabase/migrations/<timestamp>_remote_schema.sql`. That file is production history.

**Incorrect:** `supabase db pull` to "sync types" or to see what tables exist.

**Correct:** Inspect local Studio at `http://127.0.0.1:54323`. Pull only when the human asked to import a hosted schema.

Notes: Types come from `types-gen-local`, not from a pull.
