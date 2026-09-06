---
title: Component folder layout
impact: HIGH
impactDescription: Predictable files for every UI kit primitive
tags: [package, structure, components]
---

## Component folder layout

New primitives follow the folder shape of an existing primitive in the same kit. Types, variants, and the component stay adjacent.

When the kit already uses CVA, prefer this four-file split:

```
{ui-kit}/src/components/{category}/{ComponentName}/
  ├── index.ts
  ├── {ComponentName}.tsx            # forwardRef component
  ├── {ComponentName}.types.ts       # Props with VariantProps
  └── {ComponentName}.variants.ts    # cva() definitions
```

**Incorrect:** dumping a one-off `MyWidget.tsx` at package root with props and classes inline.

**Correct:** the category folder plus the four files the neighboring primitive already uses. `index.ts` re-exports the public API.

**Notes.** Feature UI that is not a shared primitive lives next to the feature screen. Mirror the same split (component / types / variants) when the feature owns non-trivial variants.
