---
title: Target refs, then locators
impact: CRITICAL
tags: [snap, refs, locators]
---

## Target refs, then locators

Default target is a ref from the latest snapshot. CSS and Playwright locator strings also work.

```bash
playwright-cli snapshot
playwright-cli click e15
playwright-cli snapshot e34
playwright-cli snapshot --depth=4
playwright-cli click "getByTestId('submit-button')"
playwright-cli generate-locator e5 --raw
```

**Incorrect:** Long CSS against RN-web generated classes.

**Correct:** Click the snapshot ref. If you need a stable locator, `generate-locator` or `getByTestId` / `getByRole` (`sel-role-testid`).

Notes: Output often points at `.playwright-cli/page-….yml`. That dir is the CLI cache, not a commit target.
