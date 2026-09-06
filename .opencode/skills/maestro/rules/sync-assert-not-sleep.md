---
title: Wait with assertions, not sleeps
impact: HIGH
impactDescription: blind waits hide slowness and still flake
tags: [sync, wait]
---

## Wait with assertions, not sleeps

`assertVisible` and `assertNotVisible` poll until the element appears or the default timer expires. That is the wait.

```yaml
- tapOn:
    id: submit
- assertVisible:
    id: home_root
```

**Incorrect:** A bare `extendedWaitUntil` with a 60s timeout used as `sleep`, or any `wait` with no element.

**Correct:** Assert the thing the user would see. Reserve `extendedWaitUntil` for a named slow step (payment, generate). Use `waitForAnimationToEnd` only when a visible control is still moving.

Notes: Inflating timeouts is `react-testing` `flake-no-timeout-inflate`.
