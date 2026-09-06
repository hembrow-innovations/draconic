---
title: Typed search params
impact: HIGH
impactDescription: Search state is a typed contract, not window.location soup
tags: [start, search, validation]
---

## Typed search params

Validate search with `validateSearch`. Read it through `Route.useSearch()`. Do not parse `window.location.search` for UI state that should survive navigation.

**Incorrect:**

```tsx
function ProductList() {
  const params = new URLSearchParams(window.location.search)
  const page = Number(params.get("page") ?? 1)
  return <List page={page} />
}
```

**Correct:**

```ts
import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"

const productSearch = z.object({
  page: z.coerce.number().int().positive().default(1),
  q: z.string().default(""),
})

export const Route = createFileRoute("/shop/products")({
  validateSearch: productSearch,
  component: ProductList,
})

function ProductList() {
  const { page, q } = Route.useSearch()
  return <List page={page} query={q} />
}
```

**Notes.** Use the schema library the repo already uses (Zod, Valibot, ArkType). Copy a sibling route. For a one-shot read inside a click handler that must not subscribe, see `rerender-defer-reads`.
