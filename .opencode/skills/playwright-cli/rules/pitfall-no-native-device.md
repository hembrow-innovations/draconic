---
title: Do not point Playwright at a device
impact: HIGH
tags: [pitfall, maestro]
---

## Do not point Playwright at a device

This CLI cannot drive Expo Go, an iOS simulator, or an Android emulator.

**Incorrect:** Attaching via adb or simctl, or opening an `exp://` URL in Playwright.

**Correct:** Use Maestro for device E2E. Use this CLI only after Expo web (or another web target) is serving HTTP.

Notes: Layer split is `stack-web-not-native`.
