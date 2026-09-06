---
title: Never commit secret keys
impact: CRITICAL
impactDescription: git history keeps leaked JWTs forever
tags: [key, secrets]
---

## Never commit secret keys

Secret keys, `service_role`, database passwords, and `SUPABASE_ACCESS_TOKEN` stay out of git.

**Incorrect:** Committing `.env`, `.env.production`, or a screenshot of `supabase status` that includes the secret.

**Correct:** Commit `.env.example` with empty values. Gitignore `.env`, `.env.local`, and `.env.*.local`. Put hosted secrets in the host's secret store.

Notes: Publishable and `anon` keys are designed to be public. Still prefer env files over hardcoding.
