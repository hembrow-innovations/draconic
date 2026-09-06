---
title: Declarative schemas only if the repo has them
impact: MEDIUM
impactDescription: mixing schemas/ and ad-hoc migrations splits the source
tags: [mig]
---

## Declarative schemas only if the repo has them

Some projects declare schema under `supabase/schemas` and generate migrations. That is optional. Do not introduce it into a migrations-only repo.

**Incorrect:** Adding `supabase/schemas` because a blog post used declarative schema.

**Correct:** Follow the folder that already exists. Migrations-only stays migrations-only.

Notes: `mig-source-of-truth`.
