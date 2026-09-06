---
title: Preview branches are hosted
impact: HIGH
impactDescription: supabase branches talks to the platform, not Docker
tags: [prod]
---

## Preview branches are hosted

Persistent preview branches and `supabase branches` live on the Supabase platform. They are not the local stack.

**Incorrect:** Creating a preview branch to try a migration.

**Correct:** Use local `db reset` for the experiment. Touch branches only when the human asked for a hosted preview.

Notes: Local is cheaper and does not spend quota.
