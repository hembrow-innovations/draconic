---
title: Hooks live in the data layer
impact: CRITICAL
impactDescription: Wrong package ownership breaks layering and RLS boundaries
tags: [layout, data-layer, layering]
---

## Hooks live in the data layer

Discover the package that owns query keys, query options, and domain mutations. Feature UI packages only consume those hooks. They do not own API, query, or mutation code.

`@project_name/react-api` (`packages/data/react-api/src/`) is one such package, not the only home.

Durable data-flow and intent notes live under `docs/`. Load **docs** and search.

**Incorrect (API owned by a feature package):**

```ts
// packages/features/vehicles/web/src/api/useVehicle.ts
export function useVehicle(id: string) {
  const supabase = useSupabase();
  return useQuery({
    queryKey: ["vehicle", id],
    queryFn: () => fetchVehicle(supabase, id),
  });
}
```

**Correct (feature imports from the data layer):**

```ts
import { useVehicle } from "@project_name/react-api/vehicles/vehicle";
```

**Notes:** An internal alias such as `@main-api/*` is for tests and tooling only when the project already has one. It is not a published package path.
