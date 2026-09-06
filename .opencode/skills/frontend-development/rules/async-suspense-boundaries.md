---
title: Stream around the wait
impact: HIGH
impactDescription: Faster first paint without async RSC as the default
tags: [async, suspense, pending, query]
---

## Stream around the wait

Start components are interactive and SSR by default. Do not invent async Server Components to stream. Use route `pendingComponent`, Router `Await`, or `useSuspenseQuery`.

**Incorrect (whole page blocked, Next-shaped async child):**

```tsx
async function Page() {
  const data = await fetchData()
  return (
    <div>
      <Sidebar />
      <DataDisplay data={data} />
    </div>
  )
}
```

**Correct (route pending):**

```ts
export const Route = createFileRoute("/dashboard")({
  loader: ({ context }) =>
    context.queryClient.ensureQueryData(dashboardQueryOptions()),
  pendingComponent: DashboardSkeleton,
  component: DashboardPage,
})
```

**Correct (defer a slow slot):**

```tsx
import { Await } from "@tanstack/react-router"
import { Suspense } from "react"

function DashboardPage() {
  const { slow } = Route.useLoaderData()
  return (
    <div>
      <Sidebar />
      <Suspense fallback={<Skeleton />}>
        <Await promise={slow}>{(data) => <DataDisplay data={data} />}</Await>
      </Suspense>
    </div>
  )
}
```

**Notes.** Skip Suspense when the data decides layout, when above-the-fold SEO copy must be in the first HTML, or when the query is tiny. Prefer `useSuspenseQuery` when the loader already prewarmed the same key. See `start-pending`.
