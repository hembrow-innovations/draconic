---
title: Auth redirects in beforeLoad
impact: HIGH
impactDescription: Stops unauthorized renders without treating the route as the data boundary
tags: [start, auth, beforeLoad]
---

## Auth redirects in beforeLoad

Gate a route with `beforeLoad`. Throw `redirect` or `notFound` there so the page never paints. Still authenticate every server function that reads or writes private data.

**Incorrect:**

```ts
export const Route = createFileRoute("/settings")({
  component: function Settings() {
    const session = useSession()
    if (!session) return <Navigate to="/login" />
    return <SettingsForm />
  },
})
```

The page loads first. A direct call to the settings server fn still works.

**Correct:**

```ts
import { createFileRoute, redirect } from "@tanstack/react-router"

export const Route = createFileRoute("/settings")({
  beforeLoad: async ({ context, location }) => {
    const session = await context.auth.getSession()
    if (!session) {
      throw redirect({
        to: "/login",
        search: { redirect: location.href },
      })
    }
    return { session }
  },
  component: SettingsForm,
})
```

**Notes.** Parent `beforeLoad` runs before children. Put shared session work on the pathless layout the repo already uses. Pair with auth middleware on the server fn. See `start-server-fn`.
