---
title: Lazy-load heavy widgets
impact: CRITICAL
impactDescription: Directly affects TTI and LCP
tags: [bundle, lazy, code-split]
---

## Lazy-load heavy widgets

Editors, charts, and other large widgets stay out of the first page chunk. Use React `lazy()` plus Vite `import()`. Route-level splits use `start-code-split`.

**Incorrect:**

```tsx
import { MonacoEditor } from "./monaco-editor"

function CodePanel({ code }: { code: string }) {
  return <MonacoEditor value={code} />
}
```

**Correct:**

```tsx
import { lazy, Suspense } from "react"

const MonacoEditor = lazy(() =>
  import("./monaco-editor").then((m) => ({ default: m.MonacoEditor })),
)

function CodePanel({ code }: { code: string }) {
  return (
    <Suspense fallback={<EditorSkeleton />}>
      <MonacoEditor value={code} />
    </Suspense>
  )
}
```

**Notes.** Always wrap `lazy()` in `Suspense`. Cross-package lazy imports use the package public export, then pick the named component. Do not use `next/dynamic`.
