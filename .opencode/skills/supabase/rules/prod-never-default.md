---
title: Local is the default target
impact: CRITICAL
impactDescription: a silent hosted call mutates production
tags: [prod]
---

## Local is the default target

Every command and env value targets the local Docker stack unless the human named a hosted project.

**Incorrect:** Opening the cloud dashboard or copying `https://<ref>.supabase.co` because the task said "use supabase".

**Correct:** Run `supabase start` and `supabase status`. Point the app at `http://127.0.0.1:54321` and the keys printed by status.

Notes: Hosted work is opt-in. See `prod-opt-in-explicit`.
