---
title: db push is a hosted migrate
impact: CRITICAL
impactDescription: applies every new local migration to the linked database
tags: [prod, migrations]
---

## db push is a hosted migrate

`supabase db push` applies `supabase/migrations` to the linked remote. It is not a local command.

**Incorrect:** `supabase db push` to "apply the migration" after `migration new`.

**Correct:** Apply locally with `supabase db reset` (or the project's reset script). Push only when the human asked to migrate staging or production.

Notes: Prefer the repo's GitHub Action when one already deploys migrations.
