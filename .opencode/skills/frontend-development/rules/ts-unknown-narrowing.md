---
title: unknown, narrowing, no as
impact: HIGH
impactDescription: any and as hide crashes until runtime
tags: [ts, unknown, narrowing]
---

## unknown, narrowing, no as

External data is `unknown`. `any` disables checking everywhere it touches. Every `as` is a crash waiting. Cast only after validation. Prefer `satisfies` over `as`.

Narrowing order: discriminant switch > `in` > `typeof` / `instanceof` > user-defined type guard > `as`.

**Incorrect:**

```ts
function parseVehicle(input: any): Vehicle {
  return input as Vehicle
}
```

**Correct:**

```ts
function isVehicle(input: unknown): input is Vehicle {
  return (
    typeof input === "object" &&
    input !== null &&
    "id" in input &&
    typeof input.id === "string"
  )
}

function parseVehicle(input: unknown): Vehicle {
  if (!isVehicle(input)) throw new Error("Invalid vehicle")
  return input
}

const routes = { home: "/", vehicles: "/vehicles" } satisfies Record<string, string>
```

**Notes.** A lying guard is worse than `as`. Name guards `isX` or `hasX`. Validate at the boundary; trust types inside. Reach for `Pick` / `Omit` / `Parameters` / `ReturnType` / `Awaited` / `typeof` before a new interface. Pass object args except on hot paths.
