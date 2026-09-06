---
title: Theme tokens live in @theme
impact: CRITICAL
impactDescription: Tailwind v4 utilities come from CSS theme, not a v3 config
tags: [tailwind, theme, tokens]
---

## Theme tokens live in @theme

Tailwind CSS v4 reads design tokens from `@theme` in CSS. Product colors, fonts, and radii belong there (or in the tokens package the repo already compiles into `@theme`). Do not add a v3 `theme.extend` palette beside it.

**Incorrect:**

```js
// tailwind.config.js
export default {
  theme: {
    extend: {
      colors: {
        primary: "#2563eb",
      },
    },
  },
}
```

```tsx
<button className="bg-blue-500 text-white">Save</button>
```

**Correct:**

```css
@import "tailwindcss";

@theme {
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --font-sans: var(--font-sans);
}
```

```tsx
<button className="bg-primary text-primary-foreground">Save</button>
```

**Notes.** Discover the existing CSS entry (`app.css`, `styles.css`, or a tokens package). `--color-*` becomes `bg-*` / `text-*` / `border-*`. Face load lives in `type-face-load`. See `token-semantic-roles`.
