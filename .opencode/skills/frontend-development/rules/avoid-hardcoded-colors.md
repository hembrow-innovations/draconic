---
title: Avoid hardcoded colors
impact: LOW
impactDescription: Stops theme and brand drift
tags: [avoid, color]
---

## Avoid hardcoded colors

No hex, rgb/rgba, or fixed Tailwind palette classes for product UI.

**Incorrect:** `bg-blue-500`, `text-[#333]`, `border-[rgb(0,0,0)]`

**Correct:** `bg-primary`, `text-foreground`, `border-border` (or the matching semantic token)

See also `token-semantic-roles` and `token-foreground-pairs`.
