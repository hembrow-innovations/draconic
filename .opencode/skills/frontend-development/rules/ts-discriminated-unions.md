---
title: Discriminated unions and constructive types
impact: HIGH
impactDescription: Optional bags let illegal UI states exist
tags: [ts, unions, branded]
---

## Discriminated unions and constructive types

Model variants with a `kind` literal so impossible states cannot be represented. Brand primitives. Build the shape so the illegal value cannot be constructed.

**Incorrect:**

```ts
type ScreenState = { loading: boolean; data?: Vehicle; error?: string }
function focusAgent(id: string) {}
function pickWinner(entries: string[]): string {
  if (entries.length === 0) throw new Error("no entries")
  return entries[0]
}
```

**Correct:**

```ts
type ScreenState =
  | { kind: "loading" }
  | { kind: "ready"; data: Vehicle }
  | { kind: "error"; error: string }

type AgentId = string & { readonly __brand: "AgentId" }
type NonEmpty<T> = [T, ...T[]]

function pickWinner(entries: NonEmpty<string>): string {
  return entries[0]
}
```

**Notes.** No optional-field bags for mode. Validate brands once at creation. Keep `T[]` while operations stay total; strengthen only where the loose type forces `!`, a cast, or a "should never happen" throw. CVA visual variants still use `VariantProps` (`ts-variant-props`).
