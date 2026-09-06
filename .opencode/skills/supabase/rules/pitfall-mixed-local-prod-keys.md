---
title: Never mix local and hosted endpoints
impact: CRITICAL
impactDescription: local key against a hosted URL fails auth or writes prod
tags: [pitfall, keys]
---

## Never mix local and hosted endpoints

The API URL and the key must come from the same project. A local key is signed for `127.0.0.1`. A hosted key is signed for `*.supabase.co`.

**Incorrect:** `SUPABASE_URL=https://abcd.supabase.co` with the publishable key from `supabase status`.

**Correct:** Local URL `http://127.0.0.1:54321` plus the local key from status. Hosted URL plus hosted key only after opt-in.

Notes: This is the usual "Invalid API key" on an otherwise healthy stack.
