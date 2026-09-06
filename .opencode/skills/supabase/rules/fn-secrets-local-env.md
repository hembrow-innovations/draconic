---
title: Local function secrets live in functions/.env
impact: MEDIUM
impactDescription: hosted secrets set is a production write
tags: [fn, secrets]
---

## Local function secrets live in functions/.env

Local functions read `supabase/functions/.env`. Hosted `supabase secrets set` writes to the linked project.

**Incorrect:** `supabase secrets set MY_KEY=...` so a local function can boot.

**Correct:** Put local values in `supabase/functions/.env` (gitignored). Do not run `secrets set` unless the human asked to configure the hosted project.

Notes: Never put a secret key in function source.
