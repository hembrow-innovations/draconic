---
title: Local work does not need a link
impact: HIGH
impactDescription: link is the usual accidental production coupling
tags: [local, prod]
---

## Local work does not need a link

Init, start, status, reset, local typegen, and function serve work on an unlinked repo.

**Incorrect:** `supabase login` as step one of "set up supabase locally".

**Correct:** Skip login and link. Add them only for an explicit hosted task.

Notes: `prod-link-gated`.
