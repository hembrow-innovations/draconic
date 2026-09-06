---
title: Add schema with migration new
impact: HIGH
impactDescription: hand-named files break timestamp order
tags: [mig]
---

## Add schema with migration new

`supabase migration new <name>` writes `supabase/migrations/<timestamp>_<name>.sql`.

**Incorrect:** Creating `supabase/migrations/add_widgets.sql` without a timestamp, or editing an old file to add a table.

**Correct:**

```bash
supabase migration new add_widgets
```

Write DDL in the new file. Reset to apply.

Notes: You can pipe SQL in. `supabase migration new add_widgets < create_widgets.sql`.
