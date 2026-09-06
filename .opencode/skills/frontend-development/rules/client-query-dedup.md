---
title: Deduplicate with Query keys
impact: HIGH
impactDescription: One request per key instead of a fetch per mount
tags: [client, query, dedup]
---

## Deduplicate with Query keys

TanStack Query dedupes by key. Two components with the same key share one request. Do not fetch inside `useEffect`.

**Incorrect:**

```tsx
function UserList() {
  const [users, setUsers] = useState([])
  useEffect(() => {
    fetch("/api/users")
      .then((r) => r.json())
      .then(setUsers)
  }, [])
}
```

**Correct:**

```ts
import { queryOptions, useQuery } from "@tanstack/react-query"

export const userListQueryOptions = () =>
  queryOptions({
    queryKey: userKeys.all,
    queryFn: () => listUsersFn(),
  })

function UserList() {
  const { data: users } = useQuery(userListQueryOptions())
  return <List users={users ?? []} />
}
```

**Notes.** Key factories, `queryOptions`, and domain mutations live in the data-layer package. Feature UI only consumes them. See `layout-layering` and `keys-factories`. Do not add SWR. Immutable config can use `staleTime: Infinity`.
