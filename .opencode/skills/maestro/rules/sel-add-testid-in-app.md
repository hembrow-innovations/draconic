---
title: Add testID in the app when selectors are brittle
impact: CRITICAL
impactDescription: hacking the flow leaves the next flow stuck
tags: [sel, testid, app]
---

## Add testID in the app when selectors are brittle

A flow that only works via `point:` or colliding text is not done. The fix is a stable `testID` on the component.

**Incorrect:** `tapOn: { point: "50%,96%" }` for a tab that the app could expose as `testID="tab-tasks"`, then calling the flow finished.

**Correct:** Add `testID` on the Pressable, TextInput, or tab. Point the flow at `id:`. Keep a `point:` only when the tree truly has no node (see `sel-point-last-resort`).

Notes: Maestro does not need an npm package. `testID` is a React Native prop.
