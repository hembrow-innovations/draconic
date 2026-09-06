---
title: Fill local env from supabase status
impact: HIGH
impactDescription: stale keys are the usual Invalid API key
tags: [key, local]
---

## Fill local env from supabase status

`supabase start` and `supabase status` print the live local URL and keys. Those values change if you wipe volumes or upgrade the CLI.

**Incorrect:** Copying keys from last month's notes, or from the cloud dashboard, into `.env.local`.

**Correct:** Run `supabase status` after start. Write the project URL and publishable key into the repo's local env file.

Notes: `disc-match-conventions` owns the variable names.
