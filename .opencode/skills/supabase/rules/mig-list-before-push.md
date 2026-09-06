---
title: List migrations before any remote apply
impact: HIGH
impactDescription: push without a list surprises staging
tags: [mig, prod]
---

## List migrations before any remote apply

`supabase migration list` shows local files versus the linked remote. Run it before any hosted apply the human asked for.

**Incorrect:** `supabase db push` with no list, hoping only one file is new.

**Correct:** `supabase migration list`, read the gap, then push only if that gap is what the human asked to deploy.

Notes: Still gated by `prod-db-push-gated`.
