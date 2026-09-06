---
title: dump --linked is production data
impact: CRITICAL
impactDescription: copies hosted rows onto disk
tags: [prod, seed]
---

## dump --linked is production data

`supabase db dump --linked` (and `--linked --data-only`) reads the hosted database. That is customer data.

**Incorrect:** Dumping production into `supabase/seed.sql` to get realistic fixtures.

**Correct:** Seed from committed fixtures or `supabase db dump --local --data-only`. Dump a hosted database only when the human asked, and never commit the result unless they said to.

Notes: Local dump is fine. Linked dump is opt-in.
