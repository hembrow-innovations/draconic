---
title: Read console and network after a miss
impact: HIGH
tags: [inspect, console, network]
---

## Read console and network after a miss

```bash
playwright-cli console
playwright-cli console warning
playwright-cli requests
playwright-cli request 5
playwright-cli highlight e5
playwright-cli show --annotate
```

**Incorrect:** Retrying the same click after a blank screen with no look at console or requests.

**Correct:** `console` and `requests` after a failed act. `highlight` to confirm the ref. `show --annotate` when the user should mark the page.

Notes: Pair with `example-debug`. Tracing is `inspect-trace-video`.
