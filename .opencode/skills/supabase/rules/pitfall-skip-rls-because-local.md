---
title: Local still needs RLS
impact: CRITICAL
impactDescription: policies missing locally ship missing
tags: [pitfall, rls]
---

## Local still needs RLS

The local stack is the same Postgres roles as hosted. A table without RLS is world-readable through the publishable key.

**Incorrect:** Leaving RLS off "until production" so seed data is easier to query.

**Correct:** Enable RLS in the same migration that creates the table. Seed with a known user if policies need an owner.

Notes: `rls-enable-every-table`.
