---
title: Interaction states and accessibility
impact: MEDIUM
impactDescription: WCAG 2.1 AA and complete control states
tags: [quality, a11y, states]
---

## Interaction states and accessibility

Interactive elements need hover, focus, active, disabled, loading, and error where those states exist. Prefer semantic HTML. Meet WCAG 2.1 AA, including ≥4.5:1 contrast for body text.

**Incorrect:**

```tsx
<div onClick={onSave} className="bg-primary text-primary-foreground">
  Save
</div>
```

**Correct:**

```tsx
<button
  type="button"
  onClick={onSave}
  disabled={isSaving}
  className={cn(
    buttonVariants({ variant: "default" }),
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
  )}
>
  {isSaving ? "Saving…" : "Save"}
</button>
```

**Notes.** Keyboard focus must be visible. Do not remove outlines without a replacement ring. Loading and error copy should be plain and specific.
