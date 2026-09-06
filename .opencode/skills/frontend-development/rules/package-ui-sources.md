---
title: UI only from platform packages
impact: HIGH
impactDescription: Stops rogue primitives and cross-surface imports
tags: [package, boundary, ui]
---

## UI only from platform packages

Presentation imports building blocks only from the kit for that surface.

- **Web and desktop:** `ui-components-web` / `ui-infra-web`
- **Mobile:** `ui-components-native` / `ui-infra-native`

Feature UI may compose those primitives next to the screen. It does not invent a parallel kit.

**Incorrect:**

```tsx
import { Button } from "@/components/Button"
import { Card } from "@acme/some-random-kit"
import { Input } from "ui-components-native"
```

**Correct:**

```tsx
import { Button } from "ui-components-web"
import { cn } from "ui-infra-web"
```

```tsx
import { Button } from "ui-components-native"
import { cn } from "ui-infra-native"
```

**Notes.** Use the workspace package names as they appear in `package.json` if they are scoped (`@acme/ui-components-web`). Desktop that shells a web SPA still uses the web kit, not a third library. Native screens never import web primitives. Extend the kit; do not fork a local `Button.tsx` in the app.
