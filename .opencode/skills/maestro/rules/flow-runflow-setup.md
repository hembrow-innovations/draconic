---
title: Share login with runFlow
impact: HIGH
impactDescription: copied launch and login drift apart
tags: [flow, runflow]
---

## Share login with runFlow

If the suite already has a subflow folder, use it. Do not paste launch + login into every file.

```yaml
appId: com.example.app
---
- runFlow:
    file: ../subflows/signin.yaml
    env:
      USER_EMAIL: ${USER_EMAIL}
- assertVisible:
    id: home_root
```

**Incorrect:** Duplicating ten tap/input steps in every journey, then updating only one copy when the login screen changes.

**Correct:** One `signin.yaml` (or whatever name the repo uses). Call it with `env` for the user. Add a new subflow only when the suite already uses that pattern.

Notes: Do not invent a POM layer the repo does not have.
