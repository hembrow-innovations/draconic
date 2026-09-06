---
title: Never reset a hosted database
impact: CRITICAL
impactDescription: a remote reset destroys production data
tags: [prod]
---

## Never reset a hosted database

`supabase db reset` recreates the **local** database from migrations and seed. There is no safe one-liner that wipes a hosted project. Do not invent one.

**Incorrect:** Searching for a remote reset, or running destructive SQL in the cloud SQL editor to "match local".

**Correct:** Reset local only. Hosted schema changes go through migrations the human asked to push.

Notes: `stop --no-backup` also destroys **local** volumes. Say so before using it.
