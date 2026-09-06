---
title: Prefetch in route loaders only
impact: MEDIUM
impactDescription: Shared query options warm cache without orphan prefetch modules
tags: [prefetch, routes, loaders]
---

## Prefetch in route loaders only

There are **no** `*.prefetch.ts` files. Prefetch in route loaders with the same `*QueryOptions` factories hooks use.

**Incorrect (orphan prefetch module):**

```ts
// example: <data-layer>/vehicles/vehicles.prefetch.ts
export async function prefetchVehicles(qc, client) {
  await qc.prefetchQuery(vehiclesQueryOptions(client));
}
```

**Correct (route loader):**

```ts
// example: apps/web/src/routes/.../vehicles/index.tsx
await context.queryClient.prefetchQuery(
  vehiclesQueryOptions(context.supabase),
);
```

**Server query client:** discover the project's helper. `createServerQueryClient()` from `@project_name/react-api/main/clients` is one (60s staleTime, no refetch on mount or focus).

**Notes:** Parallel independent prefetches with `Promise.all` when a route needs several domains.
