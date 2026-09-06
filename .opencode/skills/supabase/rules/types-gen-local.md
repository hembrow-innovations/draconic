---
title: Generate types from the local database
impact: HIGH
impactDescription: --project-id reads hosted schema and needs a token
tags: [types]
---

## Generate types from the local database

`supabase gen types typescript --local` introspects the running local database. That matches the migrations you just reset.

**Incorrect:** `supabase gen types typescript --project-id "$PROJECT_REF"` during local feature work.

**Correct:**

```bash
supabase gen types typescript --local > <path-the-repo-already-uses>
```

Start or `db start` first. Write to the file the client already imports.

Notes: `--project-id` is opt-in hosted. `prod-opt-in-explicit`.
