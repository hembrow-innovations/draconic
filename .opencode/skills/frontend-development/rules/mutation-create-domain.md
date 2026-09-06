---
title: Domain mutation helper for writes
impact: HIGH
impactDescription: Shared helper injects client/queryClient and standard invalidation
tags: [mutation, optimistic, invalidation]
---

## Domain mutation helper for writes

Discover the project's domain mutation helper. `createDomainMutation` from `@project_name/react-api/main/utils` is one such helper. It injects the client and `useQueryClient()`.

Use that helper when it exists. Do not invent a new one. If none exists, keep the mutation in the data layer and invalidate through the key factory.

**Incorrect (raw useMutation + wrong optimistic API):**

```ts
export function useCreateVehicle() {
  const client = useSupabase();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vehicle: NewVehicle) => createVehicle(client, vehicle),
    onSuccess: () => qc.invalidateQueries({ queryKey: vehicleKeys.all }),
    onOptimisticUpdate: () => {},
  });
}
```

**Correct:**

```ts
export const useCreateVehicle = createDomainMutation({
  mutationFn: (client, vehicle: NewVehicle) => createVehicle(client, vehicle),
  invalidateKeys: () => [vehicleKeys.all],
});

export const useUpdateVehicle = createDomainMutation({
  mutationFn: (client, { vehicleId, vehicle }) =>
    updateVehicle(client, vehicleId, vehicle),
  invalidateKeys: (_input, data) => [
    vehicleKeys.all,
    vehicleKeys.detail(data.id),
  ],
});
```

**Rules:**

- **`invalidateKeys(input, output)`** non-optimistic on success, optimistic on settled
- **Optimistic** `optimistic: (input) => readonly { key, update(current) }[]`. Not `onOptimisticUpdate` or `onRollback`
- Consumer can still pass `onSuccess`, `onError`, or `onSettled`
