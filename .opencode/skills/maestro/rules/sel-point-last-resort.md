---
title: Coordinate taps are last resort
impact: HIGH
impactDescription: point selectors break on device size
tags: [sel, point]
---

## Coordinate taps are last resort

`point: "50%,96%"` is a percentage of this screen. A different AVD or inset shifts it.

**Incorrect:** Using `point` as the default because Studio showed coordinates.

**Correct:** `id:`, then `text:` + `index`, then a relational selector. `point` only when the accessibility tree has no node, and leave a comment-free nearby `testID` follow-up in the app if you can add one.

Notes: Relative `%` beats raw pixels. Still treat both as debt.
