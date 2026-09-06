---
name: frontend-development
description: Frontend apps on TypeScript, pnpm, TanStack Start, React, Tailwind v4, CVA, and TanStack Query. Use when building or reviewing components, pages, routes, loaders, tokens, variants, data hooks, types, TSDoc, nested src layout, UX, DX, bundle size, waterfalls, or frontend bugs.
---

# Frontend development

Production frontend. Folds frontend-design, tanstack-ui, vercel-react-best-practices, typography, typescript-best-practices, tsdoc-reference, diagnose, tanstack-query, and codebase-design. Detail lives in `rules/<id>.md`.

## Discover first

1. Kit first. Web and desktop import from `ui-components-web` / `ui-infra-web`. Mobile from `ui-components-native` / `ui-infra-native`. Copy one neighboring primitive.
2. Find `@theme` tokens and CVA splits. Do not invent a second palette or kit.
3. Find Start routes, `createServerFn`, and data-layer key factories.
4. Nested `src/` folders. File target ≤1000 LOC, hard 1250.

Working notes go through **management**. Durable decisions go through **docs**.

## Stack

- **Tooling.** pnpm only. TypeScript only.
- **App.** TanStack Start + Router. Loaders are isomorphic.
- **Data.** `createServerFn` plus TanStack Query in the data layer.
- **Style.** Tailwind v4 `@theme` plus CVA plus shared tokens.
- **Width.** No named `max-w-{size}`. Use fractions such as `max-w-3/4`.

### Prefer

- **tool-**, **arch-**, **size-**, **package-** toolchain, nested `src/`, kit boundaries
- **start-*** file routes, server fns, isomorphic loaders
- **async-*** waterfalls, `Promise.all`, defer await
- **token-**, **tw-**, **cva-** semantic colors and `.variants.ts`
- **layout-***, **keys-***, **query-***, **mutation-*** data layer
- **bundle-*** direct imports, lazy routes

### Apply carefully

- **server-cache-react** / **server-cache-lru** when measured
- **rendering-activity** experimental
- **diag-loop** for hard bugs. Build a tight red loop first
- **principle-**, **quality-**, **ux-** raise the bar without extra chrome

### Do not introduce

- Next.js APIs, SWR, npm, yarn, bun, or JavaScript product files
- Hardcoded hex/rgb or `bg-blue-500`
- Named `max-w-sm` / `max-w-xl` utilities
- A second UI kit or Tailwind v3 `theme.extend`
- Flat `src/` dumps or files over 1250 lines
- Secrets or `process.env` at module scope in isomorphic files

## When to apply

- Building or redesigning a component, page, or layout
- Start routes, loaders, server fns, Query hooks, tokens, CVA
- TypeScript, TSDoc, package layout, or frontend diagnosis
- Bundle, waterfall, or re-render pain

## Testing, review, audit

Do not fold these. Load the matching skill and stop:

- **spec** purpose, contract, test folders
- **tdd** red-green; **vitest** unit; **react-testing** RTL
- **webapp-testing** runner pick
- **playwright-cli** web capture; **maestro** device

## Rule categories by priority

- **1 CRITICAL** Tooling + layout (`tool-`, `arch-`, `size-`, `package-`)
- **2 CRITICAL** Start (`start-`)
- **3 CRITICAL** Waterfalls (`async-`)
- **4 CRITICAL** Tokens + CVA (`token-`, `tw-`, `cva-`)
- **5 CRITICAL** Query (`layout-`, `keys-`, `query-`, `mutation-`)
- **6 HIGH** Bundle (`bundle-`)
- **7 HIGH** Server (`server-`)
- **8 HIGH** Types + TSDoc (`ts-`, `tsdoc-`)
- **9 MEDIUM** Client, quality, UX (`client-`, `quality-`, `ux-`, `type-`)
- **10 LOW** Re-render, JS, avoid, diag (`rerender-`, `rendering-`, `js-`, `avoid-`, `diag-`, `advanced-`)

## Quick reference

### 1. Tooling + layout (CRITICAL)

- `tool-pnpm-typescript` - pnpm and TypeScript only
- `arch-src-nested` - nested `src/` subfolders, never a flat `src`
- `arch-deep-modules` - small interface, lots of behaviour, real seams
- `size-file-budget` - target ≤1000 LOC, hard 1250
- `package-ui-sources` - web+desktop `ui-components-web` / `ui-infra-web`; mobile `ui-components-native` / `ui-infra-native`
- `package-component-layout` - copy the neighboring primitive folder
- `mono-public-exports` - public export, not another package's `src/`

### 2. Start (CRITICAL)

- `start-execution-model` `start-file-routes` `start-server-fn` `start-loader-query` `start-before-load` `start-search-params` `start-code-split` `start-pending`

### 3. Waterfalls (CRITICAL)

- `async-defer-await` `async-parallel` `async-dependencies` `async-api-routes` `async-suspense-boundaries`

### 4. Tokens + CVA (CRITICAL)

- `token-semantic-roles` `token-foreground-pairs` `tw-v4-theme` `cva-variants-file` `cva-cn-merge`

### 5. Query (CRITICAL)

- `layout-layering` `layout-entity-files` `keys-factories` `query-options-hooks` `mutation-create-domain` `prefetch-route-loaders` `cache-invalidation`

### 6–7. Bundle + server (HIGH)

- `bundle-barrel-imports` `bundle-dynamic-imports` `bundle-defer-third-party` `bundle-conditional` `bundle-preload`
- `server-parallel-fetching` `server-serialization` `server-cache-react` `server-cache-lru` `server-after-nonblocking`

### 8. Types + TSDoc (HIGH)

- `ts-discriminated-unions` `ts-unknown-narrowing` `ts-exhaustiveness` `ts-variant-props`
- `tsdoc-coverage` `tsdoc-edges`

### 9. Client, quality, UX (MEDIUM)

- `client-query-dedup` `client-event-listeners` `client-passive-event-listeners` `client-localstorage-schema`
- `type-face-load` `quality-typography-spacing` `quality-states-a11y` `quality-motion-theme`
- `principle-design-thinking` `principle-product` `ux-complete-states` `ux-copy-voice`

### 10. Re-render / avoid / diag (LOW)

- `rerender-*` `rendering-*` `js-*` `advanced-*`
- `avoid-hardcoded-colors` `avoid-inline-variants` `avoid-max-w-named` `avoid-decorative-noise`
- `diag-loop` - tight red loop before hypothesising

## How to use

```text
rules/package-ui-sources.md
rules/async-parallel.md
rules/keys-factories.md
```

Pick 1–N rule ids for the task. `Read` only those `rules/<id>.md` files. Do not bulk-read `rules/` or load all of `AGENTS.md` unless asked or stuck. Prefer higher-priority categories when reviewing.

Each rule: why → incorrect → correct → notes.
