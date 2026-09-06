---
title: Exhaustive variant checks
impact: HIGH
impactDescription: New union members silently skip UI branches
tags: [ts, never, switch]
---

## Exhaustive variant checks

Inline `const _exhaustive: never = x` in default arms so a new variant is a compile error.

**Incorrect:**

```ts
function label(state: ScreenState): string {
  switch (state.kind) {
    case "loading":
      return "Loading"
    case "ready":
      return state.data.name
    default:
      return "Unknown"
  }
}
```

**Correct:**

```ts
function label(state: ScreenState): string {
  switch (state.kind) {
    case "loading":
      return "Loading"
    case "ready":
      return state.data.name
    case "error":
      return state.error
    default: {
      const _exhaustive: never = state
      return _exhaustive
    }
  }
}
```

**Notes.** Do not use a silent default string. If the compiler accepts `never`, a variant is missing from the union or the switch.
