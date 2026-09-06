---
title: Open, snapshot, act, close
impact: CRITICAL
impactDescription: acting without a snapshot clicks stale or invented refs
tags: [core, open, snapshot]
---

## Open, snapshot, act, close

Default loop: open the real origin, snapshot, act by the refs it printed, snapshot again, close.

```bash
playwright-cli open http://localhost:ORIGIN
playwright-cli snapshot
playwright-cli click e15
playwright-cli snapshot
playwright-cli close
```

**Incorrect:** `click e15` from memory, or `screenshot` as the only way to decide the next click.

**Correct:** Snapshot after every navigation. Use those refs. Screenshot only when a human needs a picture (`artifact-snapshot-over-screenshot`).

Notes: Replace `ORIGIN` with the printed origin (`disc-project-url`). `open` can take the URL. Later moves use `goto`.
