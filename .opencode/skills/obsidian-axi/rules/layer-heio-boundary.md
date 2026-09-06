---
title: CLI does not own tracker meaning
impact: LOW
impactDescription: vault tasks are not tickets
tags: [layer]
---

## CLI does not own tracker meaning

This skill is the filesystem CLI. It reads, writes, patches, and moves markdown. It does not define ticket, slice, or task meaning.

**Incorrect:** Creating a vault daily note or `docs/` scratch as if that were a ticket.

**Correct:** Use this CLI for file operations. Pass `--vault docs` for committed notes. Pass `--vault .heio` when moving or patching tracker files.

Notes: Daily notes are also not tickets (`ws-daily-not-tracker`).
