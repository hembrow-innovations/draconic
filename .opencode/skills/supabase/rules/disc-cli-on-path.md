---
title: Confirm the CLI before anything else
impact: CRITICAL
impactDescription: no CLI means every later command is fiction
tags: [disc, cli]
---

## Confirm the CLI before anything else

The runner is the Supabase CLI. A missing binary cannot start Docker services or apply migrations.

```bash
npx supabase --version || supabase --version || echo "Supabase CLI not available"
```

**Incorrect:** Writing `supabase start` after the version command failed.

**Correct:** Tell the user how to install. Prefer `npm i -D supabase` (Node 20+) or `brew install supabase/tap/supabase`. Stop. Do not invent a hosted workaround.

Notes: An npm install is invoked as `npx supabase`. Pin the version in `package.json` when the repo already does.
