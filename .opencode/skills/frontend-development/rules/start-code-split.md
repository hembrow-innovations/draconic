---
title: Split route components, not loaders
impact: CRITICAL
impactDescription: Keeps critical route config in the first chunk
tags: [start, bundle, lazy, code-split]
---

## Split route components, not loaders

Critical route config (path, `validateSearch`, `beforeLoad`, `loader`, context) stays in the route file so matching and data can start immediately. The component, pending, error, and not-found UI can load later.

**Incorrect:**

```ts
export const Route = createFileRoute("/editor")({
  loader: () => import("./editor.loader").then((m) => m.loader()),
  component: EditorPage,
})
```

Splitting the loader adds a chunk wait on top of the fetch.

**Correct:**

Prefer the bundler plugin's `autoCodeSplitting` when the repo already enables it.

Otherwise split with `.lazy.tsx`:

```ts
// routes/editor.tsx
import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/editor")({
  validateSearch: editorSearch,
  loader: editorLoader,
})
```

```ts
// routes/editor.lazy.tsx
import { createLazyFileRoute } from "@tanstack/react-router"

export const Route = createLazyFileRoute("/editor")({
  component: EditorPage,
  pendingComponent: EditorSkeleton,
  errorComponent: EditorError,
})
```

**Notes.** `__root` does not split. Heavy widgets inside a page still use `lazy(() => import())`. See `bundle-dynamic-imports`.
