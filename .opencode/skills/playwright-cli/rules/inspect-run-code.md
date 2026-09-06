---
title: run-code is one function expression
impact: MEDIUM
tags: [inspect, run-code]
---

## run-code is one function expression

```bash
playwright-cli run-code "async page => await page.context().grantPermissions(['geolocation'])"
playwright-cli run-code --filename=script.js
```

The argument must be a single function. No `import`, `export`, or `require`.

**Incorrect:** Pasting a full Node script with imports into `run-code`.

**Correct:** One `async page => { … }` body, or `--filename` pointing at a file that is that function. Use it for geo, clipboard, iframes, and route handlers the short `route` flags cannot express.

Notes: Waits belong in locators (`locator.waitFor`). Do not add `networkidle` as a fix.
