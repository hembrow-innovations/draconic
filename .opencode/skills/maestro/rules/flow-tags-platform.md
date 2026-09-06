---
title: Tag flows the way the suite already tags
impact: MEDIUM
impactDescription: untagged YAML is skipped or run on the wrong OS
tags: [flow, tags]
---

## Tag flows the way the suite already tags

Many suites run `maestro test -p android --include-tags=android`. An untagged file is silent.

**Incorrect:** Adding `checkout.yaml` with no tags next to files that all declare `tags: [android]`.

**Correct:** Copy the tag list from a neighbor. Add `ios` only when the flow is meant to run there. Use the project's include/exclude flags, do not invent new tag names.

Notes: Platform-specific steps belong behind tags or `runFlow` conditions the suite already uses.
