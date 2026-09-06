---
title: Cache invalidation paths
impact: MEDIUM
impactDescription: Stale UI when mutations, realtime, or key scope disagree
tags: [cache, invalidation, realtime]
---

## Cache invalidation paths

Three ways the cache stays honest:

1. **Mutations** `invalidateKeys` on the domain mutation helper
2. **Realtime** the project's realtime query or invalidation hook (postgres_changes then invalidate)
3. **Scope** broad `domainKeys.all`, targeted `domainKeys.detail(id)`

**Incorrect (only invalidate list, leave detail stale):**

```ts
export const useUpdateVehicle = createDomainMutation({
  mutationFn: (client, { vehicleId, vehicle }) =>
    updateVehicle(client, vehicleId, vehicle),
  invalidateKeys: () => [vehicleKeys.all],
});
```

**Correct (list + detail when output has id):**

```ts
export const useUpdateVehicle = createDomainMutation({
  mutationFn: (client, { vehicleId, vehicle }) =>
    updateVehicle(client, vehicleId, vehicle),
  invalidateKeys: (_input, data) => [
    vehicleKeys.all,
    vehicleKeys.detail(data.id),
  ],
});
```

**Notes:** Prefer targeted keys when the write touches one row. Use `domainKeys.all` when list membership or many rows may change. Realtime covers multi-client updates. Mutations cover the writer's own success path. Discover the helper names. `createDomainMutation` and `useRealtimeQuery` are examples.
