---
title: Secret keys stay on the server
impact: CRITICAL
impactDescription: BYPASSRLS is full table access
tags: [key, server]
---

## Secret keys stay on the server

Secret keys (`sb_secret_...`) and `service_role` skip every RLS policy. They belong in a trusted server, an Edge Function, or a CI secret.

**Incorrect:** Putting the secret in a `VITE_`, `EXPO_PUBLIC_`, or `NEXT_PUBLIC_` variable.

**Correct:** Keep the secret out of any `PUBLIC`, `NEXT_PUBLIC_`, `EXPO_PUBLIC_`, or `VITE_` variable. Prefer no secret key at all if the app never needs to bypass RLS.

Notes: `pitfall-service-role-browser`.
