---
title: Expo Router paths are URLs
impact: HIGH
tags: [rn, router]
---

## Expo Router paths are URLs

Expo Router file routes become paths on the web origin. Tabs and stacks still push history.

```bash
playwright-cli goto http://localhost:ORIGIN/settings
playwright-cli go-back
```

**Incorrect:** Deep-linking with an `exp://` URL, or inventing `/app/(tabs)/settings` as the browser path.

**Correct:** Discover the origin, then `goto` the public path (`/settings`, `/user/42`). Use `go-back` / `go-forward` when you need the existing stack (`nav-history`).

Notes: Group segments in parentheses are not in the URL. Read `app/` and existing links before guessing.
