---
title: Start the local stack
impact: HIGH
impactDescription: no containers means no API
tags: [local, cli]
---

## Start the local stack

`supabase start` boots Postgres, Auth, Storage, PostgREST, Studio, Mailpit, and the rest.

**Incorrect:** Pointing the app at a hosted URL because start looked slow.

**Correct:** Run the project's start script, or `supabase start`, from the directory that owns `supabase/config.toml`. Wait for the credential table. Then `supabase status` if you need the keys again.

Notes: First run downloads images. Bind to localhost on untrusted networks. See the official getting-started page.
