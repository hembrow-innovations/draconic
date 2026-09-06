---
title: Point local auth at the local app
impact: HIGH
impactDescription: redirects to production bounce local sign-in
tags: [auth]
---

## Point local auth at the local app

`auth.site_url` and `auth.additional_redirect_urls` in `config.toml` are an allow-list. Local sign-in must redirect to the local origin.

**Incorrect:** Leaving site_url on the hosted app URL, or adding a wildcard.

**Correct:** Set `site_url` to the local app (default `http://localhost:3000`) and list each exact local callback you use. Restart after the edit.

Notes: Device testing needs the same origin you put in the app env. `local-device-loopback`.
