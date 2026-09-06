---
title: Do not treat this CLI as the suite
impact: LOW
tags: [layer, e2e]
---

## Do not treat this CLI as the suite

`playwright-cli` is interactive. Committed Playwright specs, if the repo has them, run through the project script or `npx playwright test`.

**Incorrect:** Turning a click-through into a new `tests/e2e/*.spec.ts` in a repo that has no Playwright runner, or skipping the existing suite to click by hand.

**Correct:** Interactive exploration with this CLI. Regression on the committed suite when it exists. If there is no suite, do not invent one. Device journeys stay on Maestro.

Notes: Fixture imports are `react-testing` `e2e-playwright-fixtures`.
