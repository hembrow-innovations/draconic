---
title: Unit tests are not Maestro
impact: LOW
impactDescription: device E2E is the wrong harness for a hook
tags: [layer, unit]
---

## Unit tests are not Maestro

Jest and RNTL cover components, hooks, and native presentation. Maestro covers device journeys.

**Incorrect:** Claiming Maestro covered a `useTasks` change, or adding a flow to assert a loading spinner variant.

**Correct:** Unit via the package's test script (`pnpm --filter mobile test`, `just test-mobile`, or whatever exists). Device journeys via the Maestro path.

Notes: Runner choice is `react-testing` `layer-choose-runner`.
