---
title: File routes own the tree
impact: CRITICAL
impactDescription: Matches Start routing instead of inventing App Router
tags: [start, router, routes]
---

## File routes own the tree

Routes live in the discovered `routes/` tree and export `createFileRoute` with the path the plugin generated. Do not invent Next.js `app/` folders, `layout.tsx` conventions, or a parallel router.

**Incorrect:**

```tsx
// app/dashboard/page.tsx
export default function Page() {
  return <Dashboard />
}
```

**Correct:**

```tsx
// src/routes/dashboard.tsx
import { createFileRoute } from "@tanstack/react-router"
import { Dashboard } from "../features/dashboard/Dashboard"

export const Route = createFileRoute("/dashboard")({
  component: Dashboard,
})
```

**Notes.** Copy the path string from a sibling route. The router plugin owns `routeTree.gen.ts`. Do not hand-edit generated route trees. Nested layouts use `_layout` / `_pathless` files the repo already uses, not a new convention.
