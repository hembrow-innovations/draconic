---
title: Route pending and error UI
impact: HIGH
impactDescription: Users see a route-shaped wait, not a blank shell
tags: [start, pending, error, suspense]
---

## Route pending and error UI

Every data-heavy route needs a pending state and an error state. Use the route APIs, not a one-off spinner in the page body that hides the failure.

**Incorrect:**

```tsx
function InvoicePage() {
  const { data, isPending, isError } = useQuery(invoiceQueryOptions(id))
  if (isPending) return <div>Loading...</div>
  if (isError) return <div>Something went wrong</div>
  return <Invoice invoice={data} />
}
```

**Correct:**

```ts
export const Route = createFileRoute("/invoices/$invoiceId")({
  loader: ({ context, params }) =>
    context.queryClient.ensureQueryData(invoiceQueryOptions(params.invoiceId)),
  pendingComponent: InvoiceSkeleton,
  errorComponent: InvoiceError,
  notFoundComponent: InvoiceMissing,
  component: InvoicePage,
})
```

```tsx
function InvoicePage() {
  const { invoiceId } = Route.useParams()
  const { data } = useSuspenseQuery(invoiceQueryOptions(invoiceId))
  return <Invoice invoice={data} />
}
```

**Notes.** Set router `defaultPendingComponent` / `defaultErrorComponent` for the shell. Per-route overrides exist for pages with a distinct skeleton. `notFound()` from a loader or server fn maps to `notFoundComponent`.
