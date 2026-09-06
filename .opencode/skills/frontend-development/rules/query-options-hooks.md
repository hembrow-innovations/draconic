---
title: Query options factories and hooks
impact: HIGH
impactDescription: Shared options enable hooks, prefetch, and consistent realtime
tags: [query, hooks, realtime, options]
---

## Query options factories and hooks

Put `{ queryKey, queryFn }` in `*.queries.ts`. Hooks call those options with the project's client hook and usually its realtime helper.

Discover those helpers. `useSupabase`, `useRealtimeQuery`, and `useRealtimeInvalidation` from `@project_name/react-api` are one project's names, not the only ones.

**Incorrect (options inlined only in the hook):**

```ts
export function useVehicles() {
  const supabase = useSupabase();
  return useQuery({
    queryKey: vehicleKeys.all,
    queryFn: () => getVehicles(supabase),
  });
}
```

**Correct (shared options + realtime hook):**

```ts
// *.queries.ts
export const vehiclesQueryOptions = (client: SupabaseClient<Database>) => ({
  queryKey: vehicleKeys.all,
  queryFn: async () => await getVehicles(client),
});

// useVehicles.hooks.ts
export function useVehicles() {
  const supabase = useSupabase();
  return useRealtimeQuery(vehiclesQueryOptions(supabase), {
    table: "vehicles",
    channel: "vehicles-changes",
  });
}
```

**Notes:** Some domains use plain `useQuery` plus a shared realtime manager instead of a per-hook realtime wrapper. Prefer that when one manager fans out many queries. Do not invent a manager if the project has none.
