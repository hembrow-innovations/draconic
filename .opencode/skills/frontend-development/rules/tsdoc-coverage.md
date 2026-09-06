---
title: TSDoc on every declaration
impact: HIGH
impactDescription: Missing why-lines hide intent from callers and agents
tags: [tsdoc, coverage]
---

## TSDoc on every declaration

Every TypeScript declaration in product `.ts` / `.tsx` carries a `/** */` block. First line is human *why*, not a restatement of the identifier or type.

**Incorrect:**

```ts
export function isAllowedRedirectUrl(url: string): boolean {
  return allowlist.has(url)
}
```

**Correct:**

```ts
/**
 * Fail-closed allowlist for auth redirect URLs (scheme + path).
 *
 * @param url - Candidate redirect URL
 * @returns True when the URL is on the allowlist
 */
export function isAllowedRedirectUrl(url: string): boolean {
  return allowlist.has(url)
}
```

**Notes.** Cover modules, types, values, members (including private), and nested named functions. Skip imports, re-exports, generated files, `it` / `test` / `describe` callbacks, and local non-function `const` / `let`. Callables take `@param`, `@typeParam`, and `@returns` when not `void`. New or touched declarations get a block. Do not mass-sweep the repo. See `tsdoc-edges`.
