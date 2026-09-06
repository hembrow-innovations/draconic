---
title: Prefer the project's E2E script
impact: CRITICAL
impactDescription: oneshot scripts own boot, install, and tags
tags: [run, script]
---

## Prefer the project's E2E script

If the repo has `just e2e-mobile`, `npm run e2e`, or a documented wrapper, that is the path. The wrapper usually boots the AVD, installs a release build, and passes `--include-tags`.

**Incorrect:** `emulator` + `expo run:android` + ad-hoc `maestro test` as the default in a repo that already has a oneshot.

**Correct:** Run the project script. Pass its documented env to select one flow. On fail, open its output dir.

Notes: Freestyle boot is `pitfall-no-freestyle-boot`. No script? Use `run-canonical-cli`.
