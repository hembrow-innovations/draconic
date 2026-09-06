---
title: Search the page with find
impact: HIGH
impactDescription: full snapshots of Expo tabs drown the next decision
tags: [core, find, snapshot]
---

## Search the page with find

`find` greps the accessibility tree without dumping the whole page.

```bash
playwright-cli find "Sign in"
playwright-cli find --regex "Sign (in|up)"
playwright-cli find --regex "/sign (in|up)/i"
```

**Incorrect:** `snapshot` with no depth on a tab navigator, then scrolling the YAML by hand.

**Correct:** `find` the copy or test id you care about. Then snapshot that subtree ref (`snap-target-refs`).

Notes: Dynamic greetings need a regex or a `testID` (`rn-testid-web`).
