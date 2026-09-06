---
title: This CLI drives a browser
impact: CRITICAL
impactDescription: Playwright cannot attach to iOS or Android
tags: [stack, rn, maestro]
---

## This CLI drives a browser

`playwright-cli` talks to Chromium, Firefox, or WebKit. Expo web and React Native Web are in scope. The native binary is not.

**Incorrect:** Pointing this CLI at Expo Go, an iOS simulator, or an Android emulator.

**Correct:** Start the web target and open that origin. Device, emulator, and sim work is Maestro.

Notes: `--mobile` is phone chrome on the web app (`rn-mobile-profile`), not a simulator.
