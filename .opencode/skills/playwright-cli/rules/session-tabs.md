---
title: Tabs stay in one profile
impact: MEDIUM
tags: [session, tabs]
---

## Tabs stay in one profile

```bash
playwright-cli tab-list
playwright-cli tab-new
playwright-cli tab-new http://localhost:ORIGIN/settings
playwright-cli tab-select 0
playwright-cli tab-close
playwright-cli tab-close 2
```

**Incorrect:** Opening a second `playwright-cli open` when you only needed another tab, then wondering why cookies diverged.

**Correct:** `tab-new` inside the current session for another route. Use a named session when you want isolated cookies (`session-named`).

Notes: Snapshot after `tab-select`. Refs do not carry across tabs.
