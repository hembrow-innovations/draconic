---
title: Seed runs on every reset
impact: HIGH
impactDescription: manual inserts vanish, and seed must be rerunnable
tags: [mig, seed]
---

## Seed runs on every reset

`supabase/seed.sql` (and any extra paths in config) runs after migrations on `db reset`. It must tolerate a fresh database.

**Incorrect:** Inserting demo rows only in Studio, or writing seed that fails on the second reset.

**Correct:** Commit idempotent seed (or seed that assumes empty tables, which reset provides). Do not dump hosted data into seed unless asked.

Notes: `prod-dump-linked-gated`.
