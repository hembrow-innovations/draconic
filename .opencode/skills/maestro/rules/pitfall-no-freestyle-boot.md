---
title: Do not freestyle device boot
impact: CRITICAL
impactDescription: ad-hoc adb and expo skip gates the oneshot owns
tags: [pitfall, run]
---

## Do not freestyle device boot

Manual `emulator`, `adb install`, and `expo start` skip release rebuilds, animation scales, IME flags, and seed gates.

**Incorrect:** Booting an AVD by hand, installing a debug bin, then `maestro test` in a repo that documents a oneshot.

**Correct:** Use the project script. If there is no script, `maestro test` against an already-running device and a release build you can name.

Notes: Do not claim "no simulator" without `adb devices` or the script's AVD evidence.
