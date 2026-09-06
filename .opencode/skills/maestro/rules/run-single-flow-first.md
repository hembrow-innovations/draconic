---
title: Run one flow while debugging
impact: HIGH
impactDescription: a full folder run hides the failing step
tags: [run, debug]
---

## Run one flow while debugging

A folder run is for the green suite. A red investigation is one YAML.

**Incorrect:** Re-running the whole directory after every selector tweak.

**Correct:** `maestro test path/to/failing.yaml`, or the project script's single-flow env (often `E2E_*_FLOW=`). Minimize further if needed (`debug-reproduce-minimize`).

Notes: After it is green, run the project suite command once to confirm you did not break neighbors.
