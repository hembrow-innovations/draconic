---
title: Complete UI states
impact: MEDIUM
impactDescription: Missing empty, loading, and error states strand the user
tags: [ux, states, empty, error]
---

## Complete UI states

Every screen that loads data ships default, empty, loading, and error. Destructive actions confirm. Success is visible without a toast-only whisper.

**Incorrect:** a table that renders nothing while loading, stays blank when the list is empty, and swallows the mutation error.

**Correct:**

```tsx
if (query.isPending) return <VehicleListSkeleton />
if (query.isError) return <VehicleListError onRetry={() => void query.refetch()} />
if (query.data.length === 0) return <VehicleListEmpty onCreate={onCreate} />
return <VehicleList rows={query.data} />
```

**Notes.** Empty copy names the next action. Error copy names what failed and what to do. Loading does not shift layout if a sibling already reserved space. Interaction states (hover, focus, disabled) live in `quality-states-a11y`. Route pending UI lives in `start-pending`.
