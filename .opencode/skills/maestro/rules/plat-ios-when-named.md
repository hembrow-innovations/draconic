---
title: iOS only when the task names it
impact: MEDIUM
impactDescription: iOS sim is slower and often not the gate
tags: [plat, ios]
---

## iOS only when the task names it

Many repos treat Android emulator as the agent default. iOS needs Xcode, a sim, and often `DEVELOPER_DIR`.

**Incorrect:** Booting a simulator for every Maestro task, or skipping Android because the change "feels iOS".

**Correct:** Run the platform the project script defaults to. Add iOS when the task says iOS, sim, or a dual-device flow needs it. Copy the project's iOS command (`just e2e-mobile-ios` or equivalent) instead of freestyle `xcrun simctl`.

Notes: Both platforms: `plat-dual-both-installed`.
