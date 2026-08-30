---
title: Slices
impact: HIGH
tags: [slices]
---

# Slices

A slice is a vertical cut that is usable or learnable on its own. Not a layer (“backend first”). Outcome-shaped.

A slice you cannot demo or learn from in one sitting is two slices. If a change cannot wait, the slice was too big.

## Shape the slice

Write done in words on `spec.md`. Then immediately write oracles. If you cannot write a `CHECK:` / `EXPECT:`, the done is still mush. Stop and sharpen before tasks.

Status `shaping` until spec + `EXPECT:` exist. Then `frozen`. Tasks do not exist until `frozen`.

## Activate

Only one slice is `active`. That slice is the only place `tasks.md` exists. Promoting the next slice to `active` means the previous one is `met` or `abandoned`.

## Close

`--reverify` → `ALL MET`, status `met`.

Or every leftover oracle has `ABANDON:` with a named next artifact (ticket id or “drop from sprint”), status `abandoned`. Abandoned is a handoff back to planning. It is not a green checkbox.
