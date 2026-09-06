---
title: Integration tests hit the local stack
impact: MEDIUM
impactDescription: mocked clients do not prove RLS
tags: [test]
---

## Integration tests hit the local stack

When the repo has database or auth integration tests, they should talk to local Supabase, not a hosted project, and not a client mock that skips RLS.

**Incorrect:** Pointing CI at the production URL, or asserting on a mocked `from().select()` as proof of access control.

**Correct:** Start local (or `supabase db start` when only Postgres is required). Use publishable credentials plus a seeded user. See `rls-test-as-user`.

Notes: Unit tests of pure functions stay mocked. `tdd` owns what a good test is.
