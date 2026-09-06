---
title: Avoid inline variant logic
impact: LOW
impactDescription: Keeps variants in CVA
tags: [avoid, cva]
---

## Avoid inline variant logic

Do not build variant class strings with ternaries or switch maps inside the component file.

**Incorrect:**

```tsx
className={variant === "ghost" ? "hover:bg-accent" : "bg-primary text-primary-foreground"}
```

**Correct:** `className={cn(buttonVariants({ variant, size }), className)}` with definitions in `.variants.ts`.

See also `cva-variants-file`.
