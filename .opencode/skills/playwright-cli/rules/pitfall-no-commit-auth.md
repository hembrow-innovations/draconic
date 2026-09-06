---
title: Never commit storage state
impact: HIGH
impactDescription: auth.json in git is a credential leak
tags: [pitfall, auth]
---

## Never commit storage state

`state-save` writes cookies and localStorage to a JSON file.

```bash
playwright-cli state-save /tmp/auth.json
playwright-cli state-load /tmp/auth.json
```

**Incorrect:** Committing `auth.json`, `*.auth-state.json`, or putting state under `docs/`.

**Correct:** Save under a gitignored tmp path. Delete the file when the session ends. Prefer a named in-memory session (`session-named`) when you do not need reuse.

Notes: After `state-load`, `goto` the origin so cookies apply (`storage-state`).
