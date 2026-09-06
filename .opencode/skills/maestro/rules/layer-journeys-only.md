---
title: Maestro is for journeys, not variants
impact: LOW
impactDescription: E2E used as a unit test is slow and opaque
tags: [layer, e2e]
---

## Maestro is for journeys, not variants

Write a flow for sign-in → home, CRUD across screens, or a deep link. Do not write one for a style variant or a hook option.

**Incorrect:** A flow whose only step is "the primary button label is Create".

**Correct:** Unit for variants and loading/empty. Maestro for a real account landing on home, or a deep link that signs out.

Notes: If the repo has no suite and the user did not ask for one, do not add Maestro. Propose `.maestro/` and wait (`disc-project-layout`).
