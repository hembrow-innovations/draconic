---
title: Hoist static JSX
impact: LOW
impactDescription: Avoids recreating static nodes every render
tags: [rendering, jsx]
---

## Hoist static JSX

Extract static JSX outside the component so React reuses the same element.

**Incorrect:**

```tsx
function LoadingSkeleton() {
  return <div className="h-20 animate-pulse bg-muted" />
}

function Container({ loading }: { loading: boolean }) {
  return <div>{loading ? <LoadingSkeleton /> : null}</div>
}
```

**Correct:**

```tsx
const loadingSkeleton = <div className="h-20 animate-pulse bg-muted" />

function Container({ loading }: { loading: boolean }) {
  return <div>{loading ? loadingSkeleton : null}</div>
}
```

**Notes.** Large static SVGs benefit most. If the project has React Compiler, the compiler hoists this and the manual extract is optional.
