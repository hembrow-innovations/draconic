---
title: Named sessions isolate storage
impact: HIGH
tags: [session, isolation]
---

## Named sessions isolate storage

`-s` isolates cookies, storage, and tabs. The default session is used when `-s` is omitted.

```bash
playwright-cli -s=seed open http://localhost:ORIGIN
playwright-cli -s=seed snapshot
playwright-cli -s=seed close
playwright-cli list
playwright-cli close-all
playwright-cli kill-all
```

**Incorrect:** Reusing the default session across two users, or leaving zombie processes after a crash.

**Correct:** Name sessions after the user or job (`seed`, `empty`). `close-all` when done. `kill-all` for leftovers. `PLAYWRIGHT_CLI_SESSION` sets a default name.

Notes: `delete-data` wipes the profile dir for that session.
