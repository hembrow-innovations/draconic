---
title: stop keeps the local database
impact: MEDIUM
impactDescription: --no-backup deletes local volumes
tags: [local, cli]
---

## stop keeps the local database

`supabase stop` shuts containers and keeps data. `supabase stop --no-backup` deletes local volumes.

**Incorrect:** `supabase stop --no-backup` as the everyday shutdown.

**Correct:** Plain `supabase stop` when you are done for the day. Use `--no-backup` only when you intend to wipe local state, and say so.

Notes: Schema you care about must already live in migrations. See `pitfall-dashboard-as-source`.
