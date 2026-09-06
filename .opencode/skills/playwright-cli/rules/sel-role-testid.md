---
title: Role first, then testID
impact: CRITICAL
impactDescription: CSS and xpath break on RN-web style hashes
tags: [sel, role, testid]
---

## Role first, then testID

When a snapshot ref is not enough, use Playwright locators. Prefer role and name. Then `getByTestId`.

```bash
playwright-cli click "getByRole('button', { name: 'Submit' })"
playwright-cli click "getByTestId('submit-button')"
playwright-cli generate-locator e5 --raw
```

**Incorrect:** `click "#main > div > div > div"` or xpath.

**Correct:** Role and accessible name when the node has them. Otherwise `getByTestId` from the app `testID` (`rn-testid-web`).

Notes: Many Pressables have no button role (`rn-pressable`).
