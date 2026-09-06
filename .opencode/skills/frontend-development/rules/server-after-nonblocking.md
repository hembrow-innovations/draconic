---
title: Fire-and-forget after the mutation
impact: MEDIUM
impactDescription: Side effects stop delaying the response
tags: [server, async, side-effects]
---

## Fire-and-forget after the mutation

Analytics, audit logs, and cache pokes that the caller does not need to confirm should not `await`. Mark them `void` and attach `.catch()`.

**Incorrect:**

```ts
export const updateUserFn = createServerFn({ method: "POST" })
  .validator(z.object({ userId: z.string() }))
  .handler(async ({ data }) => {
    await db.user.update(data.userId, data)
    await logUserAction(data.userId, "update")
    return { ok: true }
  })
```

**Correct:**

```ts
export const updateUserFn = createServerFn({ method: "POST" })
  .validator(z.object({ userId: z.string() }))
  .handler(async ({ data }) => {
    await db.user.update(data.userId, data)
    void logUserAction(data.userId, "update").catch((err) => {
      console.error("Background log failed:", err)
    })
    return { ok: true }
  })
```

**Notes.** Do not fire-and-forget work the user must see complete (billing, emails they were promised, writes they will read on the next screen). There is no Next `after()`. `void` is the Start form.
