---
title: Copy appId from existing flows
impact: HIGH
impactDescription: a guessed bundle id launches nothing
tags: [flow, appid]
---

## Copy appId from existing flows

`appId` is the Android package or iOS bundle id. Guessing `com.foo.app` wastes a boot.

**Incorrect:** Writing `appId: com.myapp` because the repo folder is `myapp`.

**Correct:** Copy the header from a neighboring YAML. If none exist, read `app.json`, `applicationId`, or `bundleIdentifier` and propose that value before writing files.

Notes: Expo Go does not use your standalone `appId` (`rn-expo-go-openlink`).
