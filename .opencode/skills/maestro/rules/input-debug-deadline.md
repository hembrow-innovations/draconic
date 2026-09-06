---
title: inputText DEADLINE means a debug build
impact: HIGH
impactDescription: longer waits do not fix DEADLINE_EXCEEDED
tags: [input, debug, flake]
---

## inputText DEADLINE means a debug build

`inputText` hitting `DEADLINE_EXCEEDED` is almost always a debuggable or Metro binary, not a slow device.

**Incorrect:** Adding `extendedWaitUntil` or sleeps around `inputText` after a deadline error.

**Correct:** Rebuild and install a release APK or IPA. If the project oneshot refuses DEBUGGABLE, use that path. Then re-run the same flow with no extra wait.

Notes: Policy for every run is `rn-release-build`. This rule is the symptom.
