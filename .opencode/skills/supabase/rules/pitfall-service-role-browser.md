---
title: Secret keys never ship in a client
impact: CRITICAL
impactDescription: service_role and sb_secret bypass RLS
tags: [pitfall, keys]
---

## Secret keys never ship in a client

Secret keys and the legacy `service_role` JWT use `BYPASSRLS`. Anyone who extracts them from a bundle owns the database.

**Incorrect:** `createClient(url, process.env.SUPABASE_SERVICE_ROLE_KEY)` in a React component, Expo app, or any `NEXT_PUBLIC_` / `EXPO_PUBLIC_` var.

**Correct:** Browser and mobile get the publishable or `anon` key. Secret keys stay on a server, an Edge Function, or CI.

Notes: New secret keys reject browser `User-Agent`s with 401. That is not a reason to embed them anyway.
