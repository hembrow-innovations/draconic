---
title: Plain copy and visible focus
impact: MEDIUM
impactDescription: Jargon and tiny targets slow every task
tags: [ux, dx, copy, a11y]
---

## Plain copy and visible focus

UI copy is the product voice. Buttons say the action. Errors say the fix. Touch targets are ≥44px. Keyboard focus is always visible. Primary action is obvious; secondary stays secondary.

**Incorrect:**

```tsx
<button className="text-[10px] px-1">Submit</button>
<p>Unhandled exception in mutationFn</p>
```

**Correct:**

```tsx
<button type="button" className={buttonVariants({ size: "default" })}>
  Save vehicle
</button>
<p className="text-destructive">Could not save. Check the name and try again.</p>
```

**Notes.** No jargon from the stack (`mutationFn`, `queryKey`, `SSR`) in user-facing strings. Developer-facing DX (typed errors, public exports, nested `src/`) stays in source and TSDoc, not on the screen. See `quality-states-a11y` and `principle-product`.
