---
title: Create functions with the CLI
impact: MEDIUM
impactDescription: hand-rolled folders miss the Deno entrypoint
tags: [fn]
---

## Create functions with the CLI

`supabase functions new <name>` writes `supabase/functions/<name>/index.ts`. Edge Functions are Deno TypeScript.

**Incorrect:** Adding a Node `express` handler under `supabase/functions/`.

**Correct:** `supabase functions new hello-world`, then edit that `index.ts`. Serve locally before any deploy.

Notes: `fn-serve-local`. Deploy stays gated.
