---
title: No spaghetti growth
impact: CRITICAL
impactDescription: Ad-hoc branches in unrelated flows are design problems
tags: [std, spaghetti, branching, control-flow]
---

## No spaghetti growth

Do not allow random spaghetti growth in existing code.

- Be highly suspicious of new ad-hoc conditionals, scattered special cases, or one-off branches inserted into unrelated flows.
- If a change adds "weird if statements in random places", treat that as a design problem, not a stylistic nit.
- Prefer pushing the logic into a dedicated abstraction, helper, state machine, policy object, or separate module.
- Call out changes that make the surrounding code harder to reason about, even if they technically work.
