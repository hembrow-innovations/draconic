---
title: Deep modules at real seams
impact: CRITICAL
impactDescription: Shallow pass-throughs spread change and block tests
tags: [arch, module, seam, depth]
---

## Deep modules at real seams

A **module** is anything with an interface and an implementation. Design **deep** modules: lots of behaviour behind a small interface, placed at a clean **seam**. Callers and tests cross that same seam.

Use these words: module, interface, implementation, depth, seam, adapter, leverage, locality. Do not say component, service, API, or boundary for this design talk.

**Incorrect:** a wide bag of optional fields and a thin wrapper that only forwards:

```ts
export function loadVehiclePage(opts: {
  id?: string
  client?: Client
  includeTrips?: boolean
  includeFuel?: boolean
}) {
  return opts.client!.vehicles.get(opts.id!)
}
```

**Correct:** a small interface that hides the work. Accept dependencies. Return results.

```ts
export async function getVehicleDetail(
  vehicles: VehicleStore,
  id: VehicleId,
): Promise<VehicleDetail> {
  return vehicles.getDetail(id)
}
```

**Notes.** Deletion test: if deleting the module makes complexity vanish, it was a pass-through. One adapter is a hypothetical seam; two adapters make it real. Do not add a seam until something varies across it. Internal seams can stay private to the module's tests.
