---
title: Keep local and hosted env files apart
impact: HIGH
impactDescription: one file with both URLs is how prod keys leak into dev
tags: [cfg, env]
---

## Keep local and hosted env files apart

Local keys belong in the gitignored local env file. Hosted keys belong in the host's secret store or a separate production env the deploy already uses.

**Incorrect:** One `.env` that switches by commenting out the local URL.

**Correct:** `.env.local` (or the repo's existing name) for `127.0.0.1` and local keys. Production env only when deploying, and never as the developer's default.

Notes: `pitfall-mixed-local-prod-keys`.
