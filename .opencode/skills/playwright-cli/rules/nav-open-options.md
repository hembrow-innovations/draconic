---
title: Browser, profile, attach, detach
impact: HIGH
tags: [nav, open, attach]
---

## Browser, profile, attach, detach

```bash
playwright-cli open --browser=chrome
playwright-cli open --browser=webkit
playwright-cli open --mobile
playwright-cli open --device="iPhone 15"
playwright-cli open --persistent
playwright-cli open --profile=/path/to/profile
playwright-cli attach --cdp=chrome
playwright-cli attach --cdp=http://localhost:9222
playwright-cli close
playwright-cli -s=msedge detach
playwright-cli delete-data
```

**Incorrect:** `detach` on a session you started with `open`, or `close` on a browser you attached to and must leave running.

**Correct:** `open` sessions end with `close`. Attach sessions end with `detach` if the external browser should stay up. Default profile is in-memory. `--persistent` writes a profile.

Notes: `--mobile` is RN-web phone chrome (`rn-mobile-profile`).
