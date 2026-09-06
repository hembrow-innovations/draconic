---
title: Defer analytics until after hydration
impact: MEDIUM
impactDescription: Third-party scripts stay out of the first paint
tags: [bundle, third-party, analytics]
---

## Defer analytics until after hydration

Logging, error tracking, and analytics do not belong in the initial graph. Load them after hydration.

**Incorrect:**

```tsx
import { Analytics } from "@some/analytics"

function App({ children }: { children: React.ReactNode }) {
  return (
    <div>
      {children}
      <Analytics />
    </div>
  )
}
```

**Correct:**

```tsx
import { useEffect } from "react"

function App({ children }: { children: React.ReactNode }) {
  useEffect(() => {
    void import("./analytics").then((m) => m.init())
  }, [])

  return <div>{children}</div>
}
```

**Notes.** Discover the tracker the repo already uses. Do not add Vercel Analytics, Sentry, or a second logger unless the project already has it. `useEffect` runs only on the client, so the dynamic import stays out of SSR.
