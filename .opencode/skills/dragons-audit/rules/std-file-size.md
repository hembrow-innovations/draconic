---
title: File size threshold (1k lines)
impact: CRITICAL
impactDescription: Block unjustified growth past 1000 lines
tags: [std, file-size, decomposition]
---

## File size threshold

Do not let a PR push a file from under 1k lines to over 1k lines without a very strong reason.

- Treat crossing 1000 lines as a strong code-quality smell by default.
- Prefer extracting helpers, subcomponents, modules, or local abstractions.
- If the diff crosses that threshold, explicitly ask whether the code should be decomposed first.
- Only waive if there is a compelling structural reason and the resulting file is still clearly organized.

**project note:** AGENTS.md targets ≤1000 LOC, hard limit 1250. Flag the 1k crossing; hard-limit breach is worse.
