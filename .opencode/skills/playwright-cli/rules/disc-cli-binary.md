---
title: Resolve playwright-cli vs npx
impact: CRITICAL
impactDescription: wrong prefix means every later command fails
tags: [disc, install]
---

## Resolve playwright-cli vs npx

The skill examples say `playwright-cli`. The binary on this machine may be the global CLI or the local Playwright package.

```bash
command -v playwright-cli
npx --no-install playwright --version
```

**Incorrect:** Running bare `playwright-cli` after `command -v` failed, or inventing `npx playwright-cli`.

**Correct:** If `playwright-cli` is on PATH, use it. If only local Playwright exists, prefix every command with `npx playwright cli`. If neither exists, install (`core-install`) then continue.

Notes: Pick one invocation and keep it for the session.
