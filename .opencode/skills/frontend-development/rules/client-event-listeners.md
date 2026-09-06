---
title: Deduplicate Global Event Listeners
impact: LOW
impactDescription: single listener for N components
tags: client, event-listeners, subscription
---

## Deduplicate Global Event Listeners

Use a module-level sentinel with `useEffect` to share one global event listener across all component instances.

**Incorrect (N instances = N listeners):**

```tsx
function useKeyboardShortcut(key: string, callback: () => void) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === key) {
        callback()
      }
    }
    window.addEventListener("keydown", handler)
    return () => window.removeEventListener("keydown", handler)
  }, [key, callback])
}
```

Each call to `useKeyboardShortcut` registers its own listener.

**Correct (N instances = 1 listener):**

```tsx
// Module-level Map to track callbacks per key
const keyCallbacks = new Map<string, Set<() => void>>()
let listenerCount = 0

function useKeyboardShortcut(key: string, callback: () => void) {
  useEffect(() => {
    if (!keyCallbacks.has(key)) {
      keyCallbacks.set(key, new Set())
    }
    keyCallbacks.get(key)!.add(callback)
    listenerCount++

    if (listenerCount === 1) {
      const handler = (e: KeyboardEvent) => {
        if (e.metaKey && keyCallbacks.has(e.key)) {
          keyCallbacks.get(e.key)!.forEach((cb) => cb())
        }
      }
      window.addEventListener("keydown", handler)
      return () => {
        window.removeEventListener("keydown", handler)
      }
    }

    return () => {
      listenerCount--
      const set = keyCallbacks.get(key)
      if (set) {
        set.delete(callback)
        if (set.size === 0) {
          keyCallbacks.delete(key)
        }
      }
    }
  }, [key, callback])
}

function Profile() {
  // Multiple shortcuts share one listener
  useKeyboardShortcut("p", () => { /* ... */ })
  useKeyboardShortcut("k", () => { /* ... */ })
}
```

One global `keydown` listener dispatches to registered callbacks. Cleanup removes the listener when no components remain.