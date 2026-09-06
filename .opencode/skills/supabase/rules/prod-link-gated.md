---
title: Do not link unless asked
impact: CRITICAL
impactDescription: link writes a project-ref and later commands follow it
tags: [prod, cli]
---

## Do not link unless asked

`supabase link --project-ref` binds this checkout to a hosted project. Later `db push`, `db pull`, and some dumps follow that binding.

**Incorrect:** `supabase link --project-ref $PROJECT_ID` during local setup.

**Correct:** Leave the repo unlinked. Local start, reset, typegen, and function serve do not need a link.

Notes: If `.temp/project-ref` already exists, do not use it unless the current task is hosted work.
