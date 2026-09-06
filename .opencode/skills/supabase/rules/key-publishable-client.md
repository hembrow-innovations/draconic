---
title: Clients use the publishable or anon key
impact: CRITICAL
impactDescription: the wrong key either fails or over-privileges
tags: [key, client]
---

## Clients use the publishable or anon key

Browser, mobile, and other public surfaces authenticate the **app**, not the user. Use the publishable key (`sb_publishable_...`) or the legacy `anon` JWT.

**Incorrect:** Passing the secret key "so RLS does not get in the way".

**Correct:**

```ts
createClient(process.env.SUPABASE_URL, process.env.SUPABASE_PUBLISHABLE_KEY)
```

Use the env names the repo already has. User identity is the Auth JWT, not the API key.

Notes: Both key generations work until the project disables the legacy pair.
