---
title: Use Activity Component for Show/Hide
impact: MEDIUM
impactDescription: preserves state/DOM
tags: rendering, activity, visibility, state-preservation
---

## Use Activity Component for Show/Hide

> **Note:** `<Activity>` is an experimental React API (Offscreen/activity boundary) not yet stable in React 19. Verify availability before adopting.

Use React's `<Activity>` to preserve state/DOM for expensive components that frequently toggle visibility.

**Usage:**

```tsx
import { Activity } from "react"

function Dropdown({ isOpen }: Props) {
  return (
    <Activity mode={isOpen ? "visible" : "hidden"}>
      <ExpensiveMenu />
    </Activity>
  )
}
```

Avoids expensive re-renders and state loss. When `mode="hidden"`, the component subtree is hidden but its state and DOM are preserved, so re-showing it is instant.

**Stable alternative (conditional rendering with CSS):**

```tsx
function Dropdown({ isOpen }: Props) {
  return (
    <div style={{ display: isOpen ? undefined : "none" }}>
      <ExpensiveMenu />
    </div>
  )
}
```

This keeps the component mounted (preserving state) while visually hiding it. Less elegant than `<Activity>` but works in all React versions.