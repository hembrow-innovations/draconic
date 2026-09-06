---
title: Expo Go launches via openLink
impact: HIGH
impactDescription: launchApp with a custom appId opens the wrong container
tags: [rn, expo]
---

## Expo Go launches via openLink

In Expo Go the app runs inside Expo's container. `launchApp` with your standalone `appId` does not open the project.

```yaml
- openLink: exp://127.0.0.1:19000
```

**Incorrect:** `launchApp` with the standalone bundle id while the device is running Expo Go.

**Correct:** Discover how this repo launches. Expo Go: `openLink` the exp URL. EAS or standalone: `launchApp` plus the real `appId` from existing flows.

Notes: Prefer a standalone or EAS build for committed E2E. Go is for local authoring only.
