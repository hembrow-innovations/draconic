---
title: Units stay on Jest and RNTL
impact: LOW
tags: [layer, jest]
---

## Units stay on Jest and RNTL

This CLI is a live browser. It is the wrong tool for a hook, a formatter, or a single component state.

**Incorrect:** Opening Expo web to assert that a hook maps three fixtures.

**Correct:** Use the repo's unit runner. Native presentation is Jest plus RNTL. Web units are Vitest or Jest plus Testing Library.

Notes: Runner choice is `react-testing` `layer-choose-runner`.
