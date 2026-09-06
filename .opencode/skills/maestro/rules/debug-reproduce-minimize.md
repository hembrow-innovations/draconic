---
title: Reproduce, then shrink the red flow
impact: HIGH
impactDescription: a 40-step YAML hides the first bad command
tags: [debug, flake]
---

## Reproduce, then shrink the red flow

Fix nothing until the same command fails twice.

1. Reproduce with `maestro test <flow>` or the project script.
2. Read the fail screenshot and `screen-hierarchy/` JSON in the output dir.
3. Cut the YAML to the smallest prefix that still fails.
4. Fix the app or the selector. Re-run until green.
5. Restore the rest of the flow and run once more.

**Incorrect:** Editing three selectors and the product in one pass after a single red.

**Correct:** One change per re-run. Keep the hierarchy dump next to the screenshot.

Notes: Artifacts: `artifact-gitignored`. Flakes: `flake-harden-not-sleep`.
