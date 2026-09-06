---
title: Per-request React.cache()
impact: MEDIUM
impactDescription: Deduplicates work inside one request
tags: [server, cache]
---

## Per-request React.cache()

Use `React.cache()` when the same helper runs more than once in one request tree. Start does not auto-memoize `fetch` the way Next does.

**Incorrect (inline object never hits):**

```ts
import { cache } from "react"

const getUser = cache(async (params: { uid: string }) =>
  db.user.findUnique({ where: { id: params.uid } }),
)

getUser({ uid: "1" })
getUser({ uid: "1" })
```

**Correct:**

```ts
import { cache } from "react"

export const getCurrentUser = cache(async (uid: string) =>
  db.user.findUnique({ where: { id: uid } }),
)
```

Call with primitives. `cache` uses `Object.is`.

**Notes.** Scope is one request. Put this in `.server.ts` and call it from server fns or other server helpers. Client cache is TanStack Query. See `client-query-dedup`.
