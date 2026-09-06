---
title: Save and load storage state
impact: MEDIUM
tags: [storage, cookies]
---

## Save and load storage state

```bash
playwright-cli state-save /tmp/auth.json
playwright-cli state-load /tmp/auth.json
playwright-cli goto http://localhost:ORIGIN
playwright-cli cookie-list
playwright-cli cookie-get session_id
playwright-cli cookie-set session_id abc123
playwright-cli localstorage-get theme
playwright-cli localstorage-set theme dark
playwright-cli sessionstorage-clear
```

**Incorrect:** `state-load` then acting on the current about:blank page, or committing the JSON.

**Correct:** Load state, then `goto` the origin so cookies apply. Keep files in tmp and delete them (`pitfall-no-commit-auth`). Prefer `-s` when you do not need a file.

Notes: IndexedDB has no first-class command. Use `run-code`.
