---
title: Production needs an explicit ask
impact: CRITICAL
impactDescription: agents treat any supabase task as a license to link
tags: [prod]
---

## Production needs an explicit ask

"Use supabase", "set up the database", and "add a table" mean local. Production, staging, and a project-ref are a different ask.

**Incorrect:** Running `supabase login` and `supabase link` because you might need types later.

**Correct:** Stay local. Ask, or stop, only when the human names production, staging, a project-ref, or a deploy.

Notes: Irreversible remote writes stay gated. Local files are reversible.
