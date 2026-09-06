---
title: Copy the repo's env names and scripts
impact: HIGH
impactDescription: invented NEXT_PUBLIC_ names miss the app's loader
tags: [disc, env]
---

## Copy the repo's env names and scripts

Projects already chose env names, a types path, and a start script. Inventing a second set leaves the app pointed at nothing.

**Incorrect:** Adding `NEXT_PUBLIC_SUPABASE_URL` to a Vite app that reads `VITE_SUPABASE_URL`.

**Correct:** Read `.env.example`, `.env.local`, and the client constructor. Reuse those names. Fill values from `supabase status`.

Notes: If no env file exists, propose one that matches the client you found, then wait.
