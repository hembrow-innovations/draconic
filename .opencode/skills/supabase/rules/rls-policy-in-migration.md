---
title: Write policies in SQL migrations
impact: HIGH
impactDescription: dashboard policies are not in git
tags: [rls, mig]
---

## Write policies in SQL migrations

Access rules belong in the same migration history as the table. Reuse existing SQL helpers when the repo has them.

**Incorrect:** Toggling policies only in Studio, or filtering in the client "so they cannot see it".

**Correct:** `create policy` in the migration. Test as `anon` and `authenticated`, not as `service_role`.

Notes: `rls-is-boundary`. `principle-rls-is-the-security-boundary` when installed.
