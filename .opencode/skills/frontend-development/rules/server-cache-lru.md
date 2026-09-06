---
title: Process-local LRU only when measured
impact: HIGH
impactDescription: Cross-request cache is easy to get wrong
tags: [server, cache, lru]
---

## Process-local LRU only when measured

`React.cache()` dies with the request. An LRU lives on the process. Use it only for hot, shared, non-user-specific lookups after you measure the hit.

**Incorrect:** assuming every instance shares memory, or caching per-user records in a global map.

**Correct:**

```ts
import { LRUCache } from "lru-cache"

const publicConfig = new LRUCache<string, PublicConfig>({
  max: 8,
  ttl: 5 * 60 * 1000,
})

export async function getPublicConfig() {
  const hit = publicConfig.get("current")
  if (hit) return hit
  const value = await loadPublicConfig()
  publicConfig.set("current", value)
  return value
}
```

**Notes.** Serverless and multi-instance deploys do not share this map. Per-user data belongs in Query or a real store (Redis) if you have one. Do not add `lru-cache` unless the repo already depends on it or the measurement is written down.
