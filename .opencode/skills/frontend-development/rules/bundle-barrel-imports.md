---
title: Import the leaf, not the barrel
impact: CRITICAL
impactDescription: 200-800ms import cost, slow Vite boots
tags: [bundle, imports, tree-shaking]
---

## Import the leaf, not the barrel

Barrel files re-export hundreds or thousands of modules. Vite still has to walk them. Import the leaf path the package documents.

**Incorrect:**

```ts
import { Check, X, Menu } from "lucide-react"
import { Button, TextField } from "@mui/material"
```

**Correct:**

```ts
import Check from "lucide-react/dist/esm/icons/check"
import X from "lucide-react/dist/esm/icons/x"
import Menu from "lucide-react/dist/esm/icons/menu"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
```

**Notes.** This project uses Vite, not Next. Do not add `optimizePackageImports`. If a kit's public export *is* the documented entry (`@acme/ui-web`), use that. Deep-import icon sets and other mega-barrels. Common offenders: `lucide-react`, `@mui/*`, `@tabler/icons-react`, `react-icons`, `lodash`, `date-fns`.
