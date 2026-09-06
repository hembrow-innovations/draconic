---
title: CVA lives in .variants.ts
impact: CRITICAL
impactDescription: Keeps variant maps testable and out of JSX
tags: [cva, variants, structure]
---

## CVA lives in .variants.ts

Define `cva()` in `{ComponentName}.variants.ts`. Components import the export. They do not inline large class maps.

**Incorrect:**

```tsx
// Button.tsx
const classes =
  variant === "destructive"
    ? "bg-error text-error-foreground"
    : "bg-primary text-primary-foreground";
```

**Correct:**

```typescript
// Button.variants.ts
import { cva } from "class-variance-authority";

export const buttonVariants = cva(
  "inline-flex items-center justify-center rounded-md font-medium transition-colors",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive: "bg-error text-error-foreground hover:bg-error/90",
        outline: "border border-input bg-background hover:bg-accent",
        ghost: "hover:bg-accent hover:text-accent-foreground",
      },
      size: {
        sm: "h-8 px-3 text-xs",
        md: "h-10 px-4 text-sm",
        lg: "h-12 px-6 text-base",
      },
    },
    defaultVariants: { variant: "default", size: "md" },
  },
);
```

**Notes.** Reuse shared size and color option types the kit already exports before inventing parallel unions. Discover those types next to existing variants.
