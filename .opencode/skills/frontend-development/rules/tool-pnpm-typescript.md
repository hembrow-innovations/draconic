---
title: pnpm and TypeScript only
impact: CRITICAL
impactDescription: Mixed package managers and JS product files fork the tree
tags: [tool, pnpm, typescript]
---

## pnpm and TypeScript only

Install and run with pnpm. Product app and package source is TypeScript (`.ts` / `.tsx`). Do not add npm, yarn, bun, or JavaScript product files.

**Incorrect:**

```bash
npm install
yarn add clsx
bun add lucide-react
```

```js
// src/utils/format.js
export function formatDate(d) {
  return d.toISOString()
}
```

**Correct:**

```bash
pnpm add clsx
pnpm --filter ui-components-web test
```

```ts
export function formatDate(d: Date): string {
  return d.toISOString()
}
```

**Notes.** Lockfile is `pnpm-lock.yaml`. This pack's installer scripts may stay `.js` / `.mjs`; that exception is this repo, not a product app. Generated or vendored files are not product source.
