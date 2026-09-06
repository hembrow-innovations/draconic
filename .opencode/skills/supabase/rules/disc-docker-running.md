---
title: Docker must already be running
impact: CRITICAL
impactDescription: start downloads images and fails without a runtime
tags: [disc, docker]
---

## Docker must already be running

The local stack is containers. Docker Desktop, OrbStack, Colima, Podman, or Rancher must be up before `supabase start`.

**Incorrect:** Retrying `supabase start` in a loop, or skipping to a hosted project because Docker is not installed.

**Correct:** Check the runtime. If it is missing, tell the user to start it and stop. Do not install Docker yourself unless asked.

Notes: First start downloads images and is slow. That is expected.
