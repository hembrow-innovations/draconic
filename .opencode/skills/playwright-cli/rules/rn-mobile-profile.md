---
title: Use --mobile for phone chrome
impact: HIGH
impactDescription: desktop viewport hides the layout bugs Expo web users hit
tags: [rn, mobile, device]
---

## Use --mobile for phone chrome

`--mobile` and `--device` change the browser viewport and user agent. They do not start a simulator.

```bash
playwright-cli open --mobile http://localhost:ORIGIN
playwright-cli open --device="iPhone 15" http://localhost:ORIGIN
```

**Incorrect:** Opening desktop Chromium and treating the layout as the phone app, or expecting `--mobile` to launch iOS.

**Correct:** Use `--mobile` (or a named `--device`) when the question is phone layout on Expo web. Native device proof is Maestro.

Notes: Snapshots are lighter in the mobile profile (`nav-open-options`).
