---
title: Do not invent the origin
impact: HIGH
tags: [pitfall, url]
---

## Do not invent the origin

Host and port come from the running process or a project skill. Common Expo numbers go stale.

**Incorrect:** Cycling `localhost:8081`, `:19006`, and `:3000` until one loads.

**Correct:** Read the start output or the project skill, then open that origin. If nothing is running, start the project's web script and wait for the printout.

Notes: Detail lives in `disc-project-url`.
