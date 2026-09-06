---
title: Keep fail evidence in gitignored output
impact: HIGH
impactDescription: process screenshots must not land in docs or apps
tags: [artifact, tmp]
---

## Keep fail evidence in gitignored output

Maestro writes screenshots, hierarchy, and logs under `~/.maestro/tests` unless the repo sets `testOutputDir` or `--test-output-dir`.

**Incorrect:** Saving debug PNGs under `docs/`, `apps/`, or any tracked tree. Committing `report.xml` from a local run.

**Correct:** On fail, open the configured output dir (often `.tmp/maestro/` when the oneshot sets it). Put agent `adb` screencaps and uiautomator dumps in a gitignored tmp path.

```bash
mkdir -p .tmp
adb exec-out screencap -p > .tmp/emulator.png
```

Notes: Default host dir is `~/.maestro/tests`. Honor the project's path when present.
