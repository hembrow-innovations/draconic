---
title: Query key factories with stable primitives
impact: CRITICAL
impactDescription: Unstable keys break cache hits, invalidation, and prefetch
tags: [keys, cache, factories]
---

## Query key factories with stable primitives

Export a key factory per entity. Prefer **stable primitives** in keys. Do not put objects in keys.

**Incorrect (inline keys, object segments):**

```ts
useQuery({
  queryKey: ["vehicles", { engineId, filters }],
  queryFn: () => getVehicles(client, { engineId, filters }),
});
```

**Correct (factory + primitives):**

```ts
// example: <data-layer>/vehicles/vehicle/vehicles.keys.ts
export const vehicleKeys = {
  all: ["vehicles"] as const,
  byEngine: (engineId: string) => ["engines", engineId, "vehicles"] as const,
  detail: (id: string) => ["vehicles", id] as const,
};
```

Complex domains nest list and detail helpers. A calendar `eventKeys.list` concatenates window bounds as primitives rather than embedding a range object.

**Notes:** Mutations and realtime invalidation must call the same factory so keys match.
