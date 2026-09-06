---
title: No named max-width scales
impact: HIGH
impactDescription: Named max-w sizes fight token themes
tags: [avoid, layout, tailwind]
---

## No named max-width scales

Do not use `max-w-{size}` utilities (`max-w-sm`, `max-w-md`, `max-w-lg`, `max-w-xl`, `max-w-2xl`, `max-w-prose`). If a max width is required, use a fraction such as `max-w-3/4`.

**Incorrect:**

```tsx
<div className="max-w-xl mx-auto">...</div>
```

**Correct:**

```tsx
<div className="max-w-3/4 mx-auto">...</div>
```

**Notes.** If the layout needs no max width, omit it. Do not invent a one-off pixel cap (`max-w-[720px]`). Copy the constraint a sibling screen already uses.
