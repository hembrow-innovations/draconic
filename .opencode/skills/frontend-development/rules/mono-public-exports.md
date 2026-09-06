---
title: Import public package exports
impact: HIGH
impactDescription: Stops deep src imports that break package boundaries
tags: [monorepo, package, exports]
---

## Import public package exports

Cross-package imports use the name and subpath in that package's `exports` map. Do not reach into another package's `src/`.

**Incorrect:**

```ts
import { Button } from "../../packages/ui-web/src/components/button/Button"
import { invoiceKeys } from "@acme/react-api/src/invoices/invoice.keys"
```

**Correct:**

```ts
import { Button } from "@acme/ui-web"
import { invoiceKeys } from "@acme/react-api/invoices"
```

**Notes.** If the symbol is not exported, add a public export in the owning package. Do not add a relative path that tunnels through the workspace. Feature UI may import the UI kit and the data layer. It does not import another feature's `src/`.
