---
title: Return only what the UI renders
impact: HIGH
impactDescription: Cuts loader and server-fn payload weight
tags: [server, serialization]
---

## Return only what the UI renders

Loader data and `createServerFn` results are serialized across the network. Map to the fields the screen uses.

**Incorrect:**

```ts
export const searchInvoicesFn = createServerFn({ method: "GET" })
  .validator(z.object({ q: z.string() }))
  .handler(async ({ data }) => db.invoice.search(data.q))
```

Each row may carry 20+ columns the table never shows.

**Correct:**

```ts
export const searchInvoicesFn = createServerFn({ method: "GET" })
  .validator(z.object({ q: z.string() }))
  .handler(async ({ data }) => {
    const rows = await db.invoice.search(data.q)
    return rows.map((r) => ({
      id: r.id,
      title: r.title,
      snippet: r.body.slice(0, 200),
    }))
  })
```

**Notes.** Same cut for `loader` return values and server-route JSON. Prefer `select` at the query when the data layer already does that. Do not return class instances, functions, or `Response` unless the fn is a raw-response handler.
