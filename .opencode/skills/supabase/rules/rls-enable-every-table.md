---
title: Enable RLS on every public table
impact: CRITICAL
impactDescription: without RLS the publishable key reads all rows
tags: [rls]
---

## Enable RLS on every public table

PostgREST exposes every table in the exposed schemas. RLS is the gate. A new table without RLS is public to anyone with the publishable key.

**Incorrect:**

```sql
create table public.notes (id uuid primary key, body text);
```

**Correct:**

```sql
create table public.notes (id uuid primary key, body text);
alter table public.notes enable row level security;
```

Add policies in the same migration.

Notes: `rls-policy-in-migration`.
