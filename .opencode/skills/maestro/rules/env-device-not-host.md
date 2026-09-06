---
title: Device loopback is not the host
impact: MEDIUM-HIGH
impactDescription: 127.0.0.1 on the guest never hits host APIs
tags: [env, network]
---

## Device loopback is not the host

The emulator or phone is another machine. `127.0.0.1` in the app env points at the guest.

**Incorrect:** `EXPO_PUBLIC_API_URL=http://127.0.0.1:54321` (or any host service) baked into the binary the device runs.

**Correct:** Android emulator to host: `10.0.2.2`. Physical device: the host LAN IP. Discover the real env file from the mobile app (`apps/mobile/.env`, `eas.json`, existing README). Rebuild after changing it.

Notes: iOS simulator can often use `localhost` for the host. Do not assume that on Android.
