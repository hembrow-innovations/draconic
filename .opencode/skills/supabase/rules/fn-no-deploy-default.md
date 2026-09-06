---
title: Function deploy is opt-in
impact: HIGH
impactDescription: same gate as prod-functions-deploy-gated, from the fn side
tags: [fn, prod]
---

## Function deploy is opt-in

Local serve is the default. Deploy publishes to the linked hosted project.

**Incorrect:** Adding a deploy step to a local "does the function run" check.

**Correct:** Serve and curl locally. Deploy only when the human asked to ship the function.

Notes: `prod-functions-deploy-gated`.
