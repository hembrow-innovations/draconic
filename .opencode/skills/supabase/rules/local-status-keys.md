---
title: status is the local credential source
impact: HIGH
impactDescription: Studio URL and keys live in status, not the cloud dashboard
tags: [local, cli]
---

## status is the local credential source

After start, `supabase status` reprints Studio, Mailpit, API, database URL, and keys.

**Incorrect:** Opening https://supabase.com/dashboard to find "the" URL for local work.

**Correct:** `supabase status`. Default Studio is `http://127.0.0.1:54323`. Default API is `http://127.0.0.1:54321`.

Notes: `key-env-from-status`.
