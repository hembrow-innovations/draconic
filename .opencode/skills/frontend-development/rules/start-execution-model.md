---
title: Isomorphic by default
impact: CRITICAL
impactDescription: Stops secret leaks and wrong-runtime bugs
tags: [start, execution, server, client]
---

## Isomorphic by default

TanStack Start includes every module in both bundles unless you constrain it. Route loaders run on the server during SSR and again on the client during navigation.

**Incorrect:**

```ts
// routes/dashboard.tsx (isomorphic)
const dbUrl = process.env.DATABASE_URL

export const Route = createFileRoute("/dashboard")({
  loader: async () => {
    const rows = await fetch(`${dbUrl}/items`).then((r) => r.json())
    return { rows }
  },
})
```

Module-scope `process.env` can inline into the client bundle. On edge runtimes it is also `undefined` at module load.

**Correct:**

```ts
// items.server.ts
export async function listItems() {
  const dbUrl = process.env.DATABASE_URL
  if (!dbUrl) throw new Error("DATABASE_URL missing")
  return fetch(`${dbUrl}/items`).then((r) => r.json())
}

// items.functions.ts
import { createServerFn } from "@tanstack/react-start"
import { listItems } from "./items.server"

export const listItemsFn = createServerFn({ method: "GET" }).handler(() =>
  listItems(),
)
```

**Notes.**

- `.functions.ts` is safe to import from components. The build replaces the handler with an RPC stub.
- `.server.ts` is only imported inside handlers.
- `createServerOnlyFn` / `createClientOnlyFn` when a utility must throw on the wrong side.
- Client-exposed env uses the `VITE_` prefix. Secrets never do.
