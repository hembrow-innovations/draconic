---
title: Do not introduce Detox or Appium
impact: CRITICAL
impactDescription: a second device stack splits the suite
tags: [pitfall, e2e]
---

## Do not introduce Detox or Appium

This pack's mobile E2E is Maestro. Detox and Appium need app instrumentation or drivers. Maestro does not.

**Incorrect:** Adding `detox` or `appium` because a blog post used them, or because Maestro is missing from PATH.

**Correct:** Install Maestro CLI if missing. Write YAML next to the existing flows. Web or desktop E2E stays on Playwright when the repo has it.

Notes: Cypress is the same "do not introduce" for web. See `react-testing` `pitfall-cypress-detox`.
