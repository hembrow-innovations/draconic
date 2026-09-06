---
title: Nested src folders
impact: CRITICAL
impactDescription: Flat src dumps are unnavigable for humans and agents
tags: [arch, layout, src, dx]
---

## Nested src folders

Source lives under `src/` in named subfolders (feature, domain, layer, or kit category). Never dump files in a flat `src/`. Apps and packages use the same rule.

**Incorrect:**

```
apps/web/src/
  App.tsx
  Button.tsx
  useVehicles.ts
  format.ts
  routes.tsx
```

**Correct:**

```
apps/web/src/
  routes/vehicles/index.tsx
  features/vehicles/list/VehicleList.tsx
  lib/dates/format.ts

packages/ui-components-web/src/
  components/actions/Button/
    Button.tsx
    Button.types.ts
    Button.variants.ts
    index.ts
```

**Notes.** Copy the neighboring tree. A new concern gets a folder, not another file at `src/` root. Barrels stay at the folder they export. See `package-component-layout` and `layout-entity-files`.
