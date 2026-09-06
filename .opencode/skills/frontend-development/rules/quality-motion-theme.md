---
title: Motion and theme
impact: MEDIUM
impactDescription: Subtle motion; themes via tokens only
tags: [quality, motion, dark-mode]
---

## Motion and theme

Use subtle transitions only (`transition-colors`, `transition-opacity`). Dark and light come from tokens, not duplicated class trees.

**Incorrect:**

```tsx
<div className="transition-all duration-700 ease-bounce hover:scale-105">
  <span className="dark:text-white text-black">Label</span>
</div>
```

**Correct:**

```tsx
<button className="bg-primary text-primary-foreground transition-colors hover:bg-primary/90">
  Label
</button>
```

**Notes.** If a control needs both themes, fix the token pair. Do not branch large style blocks on `dark:`.
