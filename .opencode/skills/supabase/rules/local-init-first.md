---
title: Init once, then reuse the folder
impact: HIGH
impactDescription: a second init splits migrations
tags: [local, cli]
---

## Init once, then reuse the folder

`supabase init` creates `supabase/config.toml` and the folder layout. It is safe to commit. Run it only when the folder is missing.

**Incorrect:** `supabase init` in an app subdirectory when the repo already has `supabase/` at the root.

**Correct:** If `supabase/config.toml` exists, skip init. If it does not, propose init and wait.

Notes: `disc-project-layout`.
