---
title: RLS is the security boundary
impact: CRITICAL
impactDescription: UI hiding and Zod are not privacy
tags: [rls]
---

## RLS is the security boundary

Every surface talks to PostgREST (or the client) directly. A hidden React field, a client filter, or a Zod parse is not access control.

**Incorrect:** `notes.filter(n => n.user_id === me)` as the only privacy rule, with RLS off or `using (true)`.

**Correct:** A policy that fails even if the UI is gone. If you cannot answer "would PostgREST still refuse the row?", the rule is in the wrong layer.

Notes: Load `principle-rls-is-the-security-boundary` when that skill is installed. Do not restate it further.
