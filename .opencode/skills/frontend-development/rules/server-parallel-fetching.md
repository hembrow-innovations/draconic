---
title: Parallel prewarm in loaders
impact: CRITICAL
impactDescription: Eliminates server-side waterfalls
tags: [server, loader, query]
---

## Parallel prewarm in loaders

Start independent `ensureQueryData` calls together. Sequential `await` in a loader adds a full round trip each.

**Incorrect:**

```ts
export const Route = createFileRoute("/dashboard")({
  loader: async ({ context }) => {
    await context.queryClient.ensureQueryData(headerQueryOptions())
    await context.queryClient.ensureQueryData(sidebarQueryOptions())
  },
  component: DashboardPage,
})
```

**Correct:**

```ts
export const Route = createFileRoute("/dashboard")({
  loader: async ({ context }) => {
    await Promise.all([
      context.queryClient.ensureQueryData(headerQueryOptions()),
      context.queryClient.ensureQueryData(sidebarQueryOptions()),
    ])
  },
  component: DashboardPage,
})
```

**Partial dependency:**

```ts
loader: async ({ context }) => {
  const sessionPromise = getSessionFn()
  const configPromise = context.queryClient.ensureQueryData(configQueryOptions())
  const session = await sessionPromise
  await Promise.all([
    configPromise,
    context.queryClient.ensureQueryData(userQueryOptions(session.userId)),
  ])
}
```

**Notes.** Query option factories live in the data layer. See `query-options-hooks`. Loaders are isomorphic, so the queryFn must call a server fn for privileged IO. See `start-loader-query`.
