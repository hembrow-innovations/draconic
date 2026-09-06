---
title: Studio is for authoring, not CI
impact: LOW
impactDescription: studio is interactive and not the committed gate
tags: [run, studio]
---

## Studio is for authoring, not CI

`maestro studio` inspects the tree and drafts YAML. The committed suite is still files plus `maestro test` or the project script.

**Incorrect:** Telling the user the suite is green because Studio played a draft. Leaving Studio-only flows that were never saved.

**Correct:** Use Studio to find an `id` or confirm a hierarchy. Write the flow into the existing folder and run it from the CLI.

Notes: Do not add Studio as a required install for agents. The CLI is enough.
