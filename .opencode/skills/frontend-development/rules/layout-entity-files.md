---
title: Per-entity file layout under the data layer
impact: CRITICAL
impactDescription: Consistent files keep keys, API, queries, and hooks discoverable
tags: [layout, files, data-layer]
---

## Per-entity file layout under the data layer

Discover the existing tree. Copy it. One example:

```
packages/data/react-api/src/
  main/          # providers, utils, hooks, clients, shared api
  vehicles/      # vehicle, fuelLogs, maintenance, trips
```

Per entity folder when that split already exists:

- **`{entity}.keys.ts`** query key factory
- **`{entity}.api.ts`** service functions
- **`{entity}.queries.ts`** `{ queryKey, queryFn }` factories
- **`{entity}.schemas.ts`** or **`.types.ts`**
- **`use{Entity}.hooks.ts`** or **`{entity}.hooks.ts`**
- **`index.ts`** barrel

**Incorrect (everything dumped in one hooks file):**

```ts
// vehicles.hooks.ts owns keys, fetch, and mutation inline
export function useVehicles() { /* fetch + key + realtime */ }
export function useCreateVehicle() { /* raw useMutation */ }
```

**Correct (split by role):**

```
vehicles/
  vehicles.keys.ts
  vehicles.api.ts
  vehicles.queries.ts
  useVehicles.hooks.ts
  index.ts
```

**Notes:** Domain folders mirror product areas that already exist. Shared providers and the domain mutation helper live under `main/` when the project uses that folder.
