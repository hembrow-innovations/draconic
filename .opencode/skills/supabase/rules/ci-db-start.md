---
title: CI starts local supabase, not production
impact: MEDIUM
impactDescription: CI with hosted credentials is a prod footgun
tags: [ci]
---

## CI starts local supabase, not production

The official environment guide uses `supabase db start` (or `supabase start`) on the runner, then typegen or tests against that.

**Incorrect:** Putting production database passwords in CI so tests "run against real data".

**Correct:** `supabase/setup-cli` plus `supabase db start` (or the project's existing workflow). Hosted `db push` stays on the deploy workflow the repo already has, after an explicit production setup.

Notes: `prod-db-push-gated`. Do not add a deploy workflow unless asked.
