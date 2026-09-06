---
title: Install on both devices before dual-device
impact: MEDIUM
impactDescription: a one-sided install fails the cast
tags: [plat, dual]
---

## Install on both devices before dual-device

A dual-device flow talks to two binaries. Missing one side looks like a Maestro bug.

**Incorrect:** `just e2e-mobile-dual` (or the repo's equivalent) with only the Android app installed.

**Correct:** Install the release build on both devices first. Set `DEVELOPER_DIR` if the iOS path needs it. Then run the documented dual command.

Notes: Skip this rule when the repo has no dual-device suite.
