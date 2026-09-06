---
title: History commands vs goto
impact: MEDIUM
tags: [nav, history]
---

## History commands vs goto

```bash
playwright-cli goto http://localhost:ORIGIN/settings
playwright-cli go-back
playwright-cli go-forward
playwright-cli reload
```

**Incorrect:** `goto` the previous URL when you need the existing Expo Router stack, including modal dismiss.

**Correct:** `goto` / `open` for an absolute URL. `go-back` / `go-forward` / `reload` when the SPA already has history you must keep.

Notes: Snapshot after each of these (`snap-after-each-act`).
