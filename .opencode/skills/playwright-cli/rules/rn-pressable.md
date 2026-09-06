---
title: Pressable is often not a button
impact: HIGH
impactDescription: getByRole('button') misses most RN taps
tags: [rn, pressable]
---

## Pressable is often not a button

`Pressable` and `TouchableOpacity` render as `div`s. They get a button role only when the app set `accessibilityRole="button"`.

**Incorrect:** `click "getByRole('button', { name: 'Save' })"` on a `Pressable` that only wraps `Text`.

**Correct:** Snapshot and click the ref, or `getByTestId`, or `getByLabel` when `accessibilityLabel` is set. If the control should be a button for users too, set `accessibilityRole="button"` in the app.

Notes: `Text` is also a `div`. `getByRole('heading')` needs `accessibilityRole="header"`.
