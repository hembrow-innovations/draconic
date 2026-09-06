---
title: Use local Studio, not cloud
impact: HIGH
impactDescription: cloud Studio is the hosted project
tags: [local]
---

## Use local Studio, not cloud

Local Studio is `http://127.0.0.1:54323` by default. The cloud dashboard is a different database.

**Incorrect:** Browsing https://supabase.com/dashboard/project/<ref>/editor to inspect local tables.

**Correct:** Open the Studio URL from `supabase status`. Table edits there still need a migration if they must survive reset.

Notes: `pitfall-dashboard-as-source`.
