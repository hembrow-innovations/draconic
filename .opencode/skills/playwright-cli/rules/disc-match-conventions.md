---
title: Copy scratch dirs and seed users
impact: CRITICAL
impactDescription: screenshots in docs/ and leftover auth waste the next run
tags: [disc, conventions]
---

## Copy scratch dirs and seed users

Repos already decide where agent dumps go and how to log in. Inventing `.tmp/` or a demo user fights that.

**Incorrect:** Writing screenshots under `docs/` or logging in as `user@example.com` when the project skill names a seed.

**Correct:** Read README, justfile, CI, and any project `playwright-cli` skill. Reuse that scratch dir and that seed. If none exist, use a gitignored tmp path and ask only if login is required and unknown.

Notes: Never commit auth files (`pitfall-no-commit-auth`).
