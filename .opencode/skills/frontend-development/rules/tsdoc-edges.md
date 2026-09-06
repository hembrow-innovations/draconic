---
title: TSDoc edges
impact: HIGH
impactDescription: Wrong tags send agents to the wrong home
tags: [tsdoc, see, link, reference]
---

## TSDoc edges

Source points outward only through TSDoc. Three edge kinds:

- **`@see`**: related symbol / API
- **`{@link …}`**: file path or `https://` URL
- **`@reference <kind>: <id>`**: durable vault truth under `docs/`

**Incorrect:**

```ts
/**
 * @see apps/web/src/lib/deep-link-auth.ts
 * @link VehicleStore
 * @reference mobile-deep-links
 */
```

**Correct:**

```ts
/**
 * Inbound recovery: {@link apps/web/src/lib/deep-link-auth.ts}.
 *
 * @see VehicleStore
 * @reference guide: mobile-deep-links
 * @reference adr: ADR-0024
 */
```

**Notes.** Planning ids (`s-*`, `ticket-*`) do not appear in source comments. Paths are posix from the monorepo root, no leading `/`. `@reference` needs a kind (`adr`, `guide`, `standard`, `architecture`, `overview`, `api`, `style`, `promise`, `purpose`, `glossary`) and a space after `:`. `@onic` only when the project uses onic; it does not replace TSDoc. `[[wikilinks]]` stay in Markdown notes.
