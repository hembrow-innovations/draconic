---
title: Foreground pairs with surfaces
impact: CRITICAL
impactDescription: Guarantees readable contrast when themes swap
tags: [token, contrast, foreground]
---

## Foreground pairs with surfaces

Each surface token has a matching `-foreground` text token. Pair them so contrast holds in light and dark without per-theme overrides.

**Incorrect:**

```tsx
<div className="bg-primary text-white">Primary block</div>
<div className="bg-error text-black">Error banner</div>
```

**Correct:**

```tsx
<div className="bg-primary text-primary-foreground">Primary block</div>
<div className="bg-error text-error-foreground">Error banner</div>
```

**Notes.** Same pattern for `secondary`, `accent`, `muted`, `warning`, `success`, `info`, and `background` / `foreground` base pairs.
