---
title: functions deploy is hosted
impact: CRITICAL
impactDescription: ships Deno functions to the project's edge
tags: [prod, functions]
---

## functions deploy is hosted

`supabase functions deploy` publishes to the linked project. Local testing does not need it.

**Incorrect:** Deploying `hello-world` so you can curl it.

**Correct:** `supabase functions serve hello-world` and curl `http://127.0.0.1:54321/functions/v1/hello-world`. Deploy only when the human asked.

Notes: `fn-serve-local` owns the local path.
