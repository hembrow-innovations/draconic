---
title: Start promises early in server fns
impact: CRITICAL
impactDescription: 2-10× improvement
tags: [async, server-fn, waterfalls]
---

## Start promises early in server fns

In `createServerFn` handlers and server routes, start independent work immediately. Await later.

**Incorrect (config waits for auth):**

```ts
import { createServerFn } from "@tanstack/react-start"

export const loadDashboardFn = createServerFn({ method: "GET" }).handler(
  async () => {
    const session = await auth()
    const config = await fetchConfig()
    const data = await fetchData(session.user.id)
    return { data, config }
  },
)
```

**Correct:**

```ts
import { createServerFn } from "@tanstack/react-start"

export const loadDashboardFn = createServerFn({ method: "GET" }).handler(
  async () => {
    const sessionPromise = auth()
    const configPromise = fetchConfig()
    const session = await sessionPromise
    const [config, data] = await Promise.all([
      configPromise,
      fetchData(session.user.id),
    ])
    return { data, config }
  },
)
```

**Server route variant:**

```ts
import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/api/dashboard")({
  server: {
    handlers: {
      GET: async () => {
        const sessionPromise = auth()
        const configPromise = fetchConfig()
        const session = await sessionPromise
        const [config, data] = await Promise.all([
          configPromise,
          fetchData(session.user.id),
        ])
        return Response.json({ data, config })
      },
    },
  },
})
```

**Notes.** Partial dependency graphs use `better-all`. See `async-dependencies`.
