---
title: Trace to debug. Video to show
impact: LOW
tags: [inspect, trace, video]
---

## Trace to debug. Video to show

```bash
playwright-cli tracing-start
playwright-cli tracing-stop
playwright-cli video-start /tmp/flow.webm
playwright-cli video-chapter "Login" --duration=2000
playwright-cli video-stop
```

Traces land under `traces/`. They include DOM, network, and screenshots.

**Incorrect:** Recording a hero video to figure out why a click missed.

**Correct:** `tracing-start` before the failing step when you need a replay. Video is for a human demo. Prefer snapshot plus console for the debug loop.

Notes: Start the browser before `video-start`. Put files in gitignored tmp (`artifact-gitignored`).
