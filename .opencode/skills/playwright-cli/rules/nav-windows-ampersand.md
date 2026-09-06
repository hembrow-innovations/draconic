---
title: Escape ampersands in Windows URLs
impact: LOW
impactDescription: query strings get truncated at the first &
tags: [nav, windows]
---

## Escape ampersands in Windows URLs

`cmd.exe` and PowerShell treat `&` as a command separator.

**Incorrect:** `playwright-cli goto "https://example.com/?a=1&b=2"` in cmd.exe.

**Correct:** On cmd.exe, escape as `^&` inside the quoted URL. In PowerShell, use `playwright-cli --% goto "https://example.com/?a=1&b=2"`. On macOS and Linux, quote the URL normally.

Notes: Expo Router query params hit this on Windows agents.
