---
title: Restart after config.toml edits
impact: HIGH
impactDescription: running containers keep the old ports and auth flags
tags: [cfg]
---

## Restart after config.toml edits

`supabase/config.toml` is the local project config. Changes apply on a fresh start.

**Incorrect:** Editing `api.port` or `auth.site_url` and expecting the running stack to notice.

**Correct:** Edit `config.toml`, then `supabase stop` and `supabase start`. Do not change ports unless the repo already needed a different bind.

Notes: Commit `config.toml`. It is not a secret file.
