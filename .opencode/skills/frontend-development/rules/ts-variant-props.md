---
title: Variant props from CVA
impact: MEDIUM
impactDescription: Component props stay in lockstep with the variant map
tags: [typescript, cva, props]
---

## Variant props from CVA

Type variant and size from `VariantProps<typeof variants>`. Do not hand-write a parallel union that can drift.

**Incorrect:**

```ts
type ButtonProps = {
  variant?: "default" | "destructive" | "ghost"
  size?: "sm" | "md" | "lg"
} & React.ButtonHTMLAttributes<HTMLButtonElement>
```

**Correct:**

```ts
import type { VariantProps } from "class-variance-authority"
import type { buttonVariants } from "./Button.variants"

export type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof buttonVariants> & {
    className?: string
  }
```

**Notes.** Reuse shared size and color option types the kit already exports before adding a new union. Discriminated unions for component *mode* (icon-only vs labeled) belong in the types file, not as optional bags. Broader type rules live in `ts-discriminated-unions` and `ts-unknown-narrowing`.
