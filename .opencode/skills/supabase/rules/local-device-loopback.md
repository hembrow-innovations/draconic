---
title: Devices cannot use 127.0.0.1 of the host
impact: HIGH
impactDescription: emulators treat localhost as themselves
tags: [local, mobile]
---

## Devices cannot use 127.0.0.1 of the host

Android emulators, iOS devices, and physical phones do not share the host's loopback. Edge Functions inside Docker also cannot reach `localhost` Postgres.

**Incorrect:** `EXPO_PUBLIC_SUPABASE_URL=http://127.0.0.1:54321` on an Android emulator.

**Correct:** Android emulator uses `http://10.0.2.2:54321`. A physical device uses the host LAN IP. An Edge Function talking to local Postgres uses `host.docker.internal`. iOS Simulator can use localhost.

Notes: `auth.site_url` and redirect allow-lists must include that same origin.
