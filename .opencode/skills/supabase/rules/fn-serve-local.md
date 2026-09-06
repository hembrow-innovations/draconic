---
title: Serve functions on the local API
impact: HIGH
impactDescription: deploy is not how you try a function
tags: [fn, local]
---

## Serve functions on the local API

`supabase functions serve <name>` runs the function at `http://127.0.0.1:54321/functions/v1/<name>` with hot reload.

**Incorrect:** `supabase functions deploy` to see a log line.

**Correct:**

```bash
supabase start
supabase functions serve hello-world
curl -i -X POST http://127.0.0.1:54321/functions/v1/hello-world \
  -H "apikey: $SUPABASE_PUBLISHABLE_KEY" \
  --data '{"name":"Functions"}'
```

Notes: Functions talking to local Postgres use `host.docker.internal`, not `localhost`.
