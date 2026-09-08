---
title: Review artifacts
impact: MEDIUM
impactDescription: Write HTML and Markdown reports in the OS temp directory
tags: [output, docs, artifacts]
---

## Review artifacts

Write two files in the OS temp directory. Nothing lands in the repo unless the user asks. Do not route through other skills.

Resolve the temp dir from `$TMPDIR`, falling back to `/tmp` (or `%TEMP%` on Windows). Write:

1. `<tmpdir>/dragons-audit-<timestamp>.html` — visual report. Open it (`open` on macOS, `xdg-open` on Linux, `start` on Windows) and tell the user the absolute path.
2. `<tmpdir>/dragons-audit-<timestamp>.md` — the same findings in Markdown.

See `references/html-report.md` for the HTML scaffold. Do not write process scratch into tracked product paths unless asked.
