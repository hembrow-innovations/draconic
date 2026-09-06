---
title: Merge CVA with package cn()
impact: CRITICAL
impactDescription: Single class-merge path per package
tags: [cva, cn, className]
---

## Merge CVA with package cn()

Components call the package-local `cn()` helper to merge variant output with caller `className`. Do not pull a random `clsx` or `twMerge` from elsewhere. Discover the helper path from a neighboring primitive.

**Incorrect:**

```tsx
import clsx from "clsx";
import { buttonVariants } from "./Button.variants";

export const Button = ({ className, variant, size, ...props }) => (
  <button className={clsx(buttonVariants({ variant, size }), className)} {...props} />
);
```

**Correct:**

```tsx
import { cn } from "../utils"; // or the package alias the kit already exports
import { buttonVariants } from "./Button.variants";

export const Button = ({ className, variant, size, ...props }) => (
  <button className={cn(buttonVariants({ variant, size }), className)} {...props} />
);
```

**Notes.** Native packages use their own `cn` path. Match the package you are editing. Do not cross-import web utils into native.
