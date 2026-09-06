---
title: Local storage is local disk
impact: LOW
impactDescription: hosted object URLs do not serve local files
tags: [store, local]
---

## Local storage is local disk

The local stack stores objects on the machine. Public URLs use the local API origin, not `*.supabase.co`.

**Incorrect:** Hardcoding a hosted storage public URL in a component while the rest of the app is local.

**Correct:** Build object URLs from the same `SUPABASE_URL` the client uses. Keep local and hosted files in their own projects.

Notes: `pitfall-mixed-local-prod-keys`.
