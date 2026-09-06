---
title: Mock network with route
impact: MEDIUM
tags: [net, route]
---

## Mock network with route

```bash
playwright-cli route "https://api.example.com/**" --body='{"mock": true}'
playwright-cli route "**/*.jpg" --status=404
playwright-cli route "**/api/users" --body='[]' --content-type=application/json
playwright-cli route-list
playwright-cli unroute "**/*.jpg"
playwright-cli unroute
```

**Incorrect:** Editing app source to stub fetch for a one-off interactive check.

**Correct:** `route` the API host the web app already calls. Discover that host from the repo. `unroute` when the mock should end.

Notes: Conditional mocks, delays, and aborts go through `run-code` (`inspect-run-code`).
