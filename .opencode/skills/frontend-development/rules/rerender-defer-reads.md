---
title: Read search on demand
impact: MEDIUM
impactDescription: Avoids subscribing a button to every search change
tags: [rerender, search, router]
---

## Read search on demand

Do not subscribe to search params when the value is only used inside a click handler.

**Incorrect:**

```tsx
function ShareButton({ chatId }: { chatId: string }) {
  const { ref } = Route.useSearch()

  const handleShare = () => {
    shareChat(chatId, { ref })
  }

  return <button onClick={handleShare}>Share</button>
}
```

Every search change re-renders the button.

**Correct:**

```tsx
function ShareButton({ chatId }: { chatId: string }) {
  const handleShare = () => {
    const params = new URLSearchParams(window.location.search)
    shareChat(chatId, { ref: params.get("ref") })
  }

  return <button onClick={handleShare}>Share</button>
}
```

**Notes.** If the value drives render (filters, pagination, selected tab), use `Route.useSearch()` and `validateSearch`. See `start-search-params`. Do not use Next `useSearchParams`.
