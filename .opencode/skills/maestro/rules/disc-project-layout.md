---
title: Discover flows and appId at runtime
impact: CRITICAL
impactDescription: invented paths miss the real suite
tags: [disc, layout]
---

## Discover flows and appId at runtime

Do not assume `.maestro/`. Search the repo.

1. Look for `.maestro/`, `maestro/`, `e2e/maestro/`, `tests/e2e/mobile/`.
2. Grep YAML for `appId:`, `tapOn:`, `launchApp`.
3. Read `app.json` or `app.config.*` only as a hint. Existing flows win.

**Incorrect:** Creating `e2e/login.yaml` in a repo whose suite already lives under `tests/e2e/mobile/`.

**Correct:** Open a neighboring flow and reuse its folder, `appId`, tags, and launch args.

Notes: If nothing exists, propose `.maestro/` and wait. See `disc-match-conventions`.
