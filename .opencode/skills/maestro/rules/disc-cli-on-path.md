---
title: Confirm Maestro CLI before anything else
impact: CRITICAL
impactDescription: no CLI means every later command is fiction
tags: [disc, cli]
---

## Confirm Maestro CLI before anything else

Flows are YAML. The runner is a host binary. If `maestro` is not on PATH, stop.

```bash
which maestro || echo "Maestro CLI not on PATH. Install from https://maestro.mobile.dev"
maestro --version
```

**Incorrect:** Writing or "running" flows after `which maestro` failed.

**Correct:** Tell the user how to install from https://maestro.mobile.dev (or the project's documented install). Do not invent a workaround with Detox or Appium.

Notes: A project script that wraps Maestro still needs the binary.
