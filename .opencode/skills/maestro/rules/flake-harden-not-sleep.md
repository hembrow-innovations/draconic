---
title: Harden selectors and sync, not sleeps
impact: HIGH
impactDescription: long sleeps make reds rarer, not greener
tags: [flake, sync]
---

## Harden selectors and sync, not sleeps

A flake is a bad id, colliding text, a covered control, a debug binary, or leftover state. It is not a slow assertion.

**Incorrect:** `extendedWaitUntil` with a 60s timeout around a tap that sometimes hits the wrong "Tasks".

**Correct:** Give the control a `testID`. Wait with `assertVisible`. Dismiss the keyboard with Enter. Use a release build. Clear state at launch.

Notes: Quarantine only if the repo has an issue-plus-expiry path. See `react-testing` `flake-quarantine-with-issue`.
