---
title: Run Maestro against a release build
impact: HIGH
impactDescription: debug bins hang input and lie about speed
tags: [rn, build]
---

## Run Maestro against a release build

Maestro drives the shipped binary. A Metro debug install makes `inputText` hang and animations lie.

**Incorrect:** `maestro test` against `expo start` or a DEBUGGABLE APK, then padding waits.

**Correct:** Use the project's mobile E2E script if it rebuilds release. Otherwise install a release APK or IPA, then `maestro test`. Discover the variant from the existing script, not from habit.

Notes: Symptom of the wrong binary is `input-debug-deadline`.
