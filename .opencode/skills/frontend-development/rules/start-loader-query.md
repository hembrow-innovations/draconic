---
title: Loaders are isomorphic
impact: CRITICAL
impactDescription: Keeps secrets off the client and prewarms Query
tags: [start, loader, query]
---

## Loaders are isomorphic

A route `loader` runs on the server during SSR and on the client during navigation. Use it to prewarm TanStack Query. Put secrets and privileged IO in `createServerFn`, then call that fn from the loader.

**Incorrect:**

```ts
export const Route = createFileRoute("/invoices")({
  loader: async () => {
    const rows = await db.invoice.findMany()
    return { rows }
  },
})
```

`db` ships to the client because the loader is isomorphic.

**Correct:**

```ts
export const Route = createFileRoute("/invoices")({
  loader: async ({ context }) => {
    await context.queryClient.ensureQueryData(invoiceListQueryOptions())
  },
  component: InvoiceListPage,
})
```

```ts
// invoiceListQueryOptions queryFn calls listInvoicesFn (a createServerFn)
```

**Notes.**

- Components read cache with `useQuery` / `useSuspenseQuery`, not by treating loader data as a second store, unless the repo already returns loader data that way.
- Prefetch, keys, and mutations live in `prefetch-route-loaders`, `keys-factories`, and `mutation-create-domain`. This rule only places the prewarm.
- Start promises immediately. See `server-parallel-fetching`.
