---
title: Semantic color roles
impact: CRITICAL
impactDescription: Keeps brand and state colors consistent across surfaces
tags: [token, color, tailwind]
---

## Semantic color roles

Every fill, border, and emphasis color comes from the theme token set. Raw palette utilities and hardcoded values drift from light/dark and break brand consistency.

Roles:

- **`primary`**: primary actions, brand emphasis
- **`secondary`**: secondary actions
- **`accent`**: highlights, decorative
- **`muted`**: subdued / disabled
- **`error`**: destructive / error
- **`warning`**: caution
- **`success`**: positive
- **`info`**: neutral informational

**Incorrect:**

```tsx
<button className="bg-blue-500 text-white">Save</button>
<span className="text-[#333]">Label</span>
```

**Correct:**

```tsx
<button className="bg-primary text-primary-foreground">Save</button>
<span className="text-muted-foreground">Label</span>
```

**Notes.** Prefer the role that matches intent (`error` for destructive), not the one that "looks right" in one theme.
