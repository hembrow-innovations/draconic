---
title: Daily notes are not tickets
impact: MEDIUM
impactDescription: daily append is not a task
tags: [ws, daily]
---

## Daily notes are not tickets

Some vaults have daily notes. A vault daily note is not a ticket or a task.

**Incorrect:** `obsidian-axi daily append --content "- [ ] ship it"` as project work.

**Correct:** Use this CLI on the note the user named. Do not treat daily notes as the work queue.

Notes: The filesystem CLI may not even expose `daily`. If a dest vault uses daily notes as a personal journal, still keep project work in its tracker files.
