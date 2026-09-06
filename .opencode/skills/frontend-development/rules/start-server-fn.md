---
title: createServerFn is the server boundary
impact: CRITICAL
impactDescription: Typed RPC instead of ad-hoc API routes
tags: [start, server-fn, validation]
---

## createServerFn is the server boundary

Server-only work (DB, secrets, privileged APIs) goes through `createServerFn`. Validate input. Static-import the fn. Auth the handler itself.

**Incorrect:**

```ts
// component
const user = await fetch("/api/users/" + id).then((r) => r.json())
```

```ts
export const getUser = createServerFn({ method: "GET" }).handler(
  async (id: string) => db.user.findUnique({ where: { id } }),
)
```

Raw handler args skip validation. Dynamic `import()` of a server fn also breaks the RPC stub.

**Correct:**

```ts
// users.functions.ts
import { createServerFn } from "@tanstack/react-start"
import { z } from "zod"
import { findUserById } from "./users.server"

export const getUser = createServerFn({ method: "GET" })
  .validator(z.object({ id: z.string() }))
  .handler(async ({ data }) => findUserById(data.id))
```

```ts
const user = await getUser({ data: { id } })
```

**Notes.**

- Handler input is `{ data }`. Return values must be serializable (`strict: true` is the default).
- Call from loaders, event handlers, or `useServerFn()`.
- `beforeLoad` is UX. It is not the data boundary. Apply auth middleware or an in-handler check on every private fn.
- Public HTTP for other clients uses a server route (`createFileRoute` + `server.handlers`), not a server fn.
