---
title: Do not edit an applied migration
impact: HIGH
impactDescription: rewriting history desyncs local, CI, and hosted
tags: [mig]
---

## Do not edit an applied migration

Once a timestamped file has been applied (local reset, teammate pull, or hosted push), treat it as immutable.

**Incorrect:** Adding a column to last week's `20240101_create_widgets.sql`.

**Correct:** `supabase migration new add_widget_color` with `alter table`. Reset local to apply.

Notes: Unapplied files you just created in this session may still be edited.
