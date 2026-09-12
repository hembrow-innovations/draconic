---
name: website
description: Website public Learn and Reference in the sibling draconic-web repo. Use when editing teaching markdown, Learn, Reference, shipped or not-yet fences, site chrome, generate-website, or the docs site. React/Tailwind/CVA: load frontend-development. Not the docs/ vault.
---

# Website

Public language site. The tree is the sibling `draconic-web` repo (`../draconic-web` from this language checkout, or `DRACONIC_WEB`). Teaching copy lives in `content/`. The Start app lives in nested `src/`. Load **frontend-development** for React, Tailwind v4, CVA, and Start craft. This tree owns local primitives under `src/components` and `src/features`. There is no `ui-components-web` kit and no TanStack Query.

Purpose, IA, and fence rules stay in this language repo at `docs/specs/draconic/public-site/`. Vault-versus-site is ADR 0010. Start presentation is ADR 0013. Load **spec** and **docs** for those; this skill owns the site tree.

## Discover first

1. Find teaching markdown. Law is `content/<slug>.md` with `title`, `section`, and `status`. Done when the Vite glob, Vitest catalog, and website-fences checker read only `content/`, and the package root has no teaching `.md`.
2. Copy a neighboring route and primitive under `src/`. Home at `/` is JSX. Learn and Reference chapters load markdown through `DocsShell`, `LearnPage`, or `ReferencePage`. Done when the new file sits beside its neighbor in the same folder shape (`Component.tsx`, `.types.ts`, `.variants.ts`, `index.ts`).
3. Load **frontend-development** for tokens, CVA, nested `src/`, and Start file routes. Hex lives in `src/styles/theme.css`. Done when the sitting uses those rules instead of inventing a second palette or kit.

## Tree

- **content/**: teaching markdown. One file per slug. Slug is the basename. Keep this folder flat so `loadMarkdownPage` can keep using the stem.
- **src/routes/**: file routes. `routeTree.gen.ts` is generated. Add the path to `prerender.pages` in `vite.config.ts` (`crawlLinks` is false).
- **src/components/** and **src/features/**: chrome and Learn/Reference/home UI.
- **src/lib/content/**: loader and renderer, not teaching copy. Point the glob at `../../../content/*.md`.
- **src/tests/**: Vitest source contracts. Run `pnpm --dir ../draconic-web exec vitest run <file-stem>` from the language root. The package `test` script runs search only.
- **package root**: `package.json`, `pnpm-lock.yaml`, `vite.config.ts`, `tsconfig.json`, `.gitignore`.

Live HTML is the Start prerender. Author markdown. Generated HTML is gitignored. Markdown may keep `install.html` style hrefs; `toAppHref` rewrites them.

Teaching copy stays out of `src/` so the loader module `src/lib/content/` does not share a folder with pages.

## Add a page

For a Learn chapter `new-chapter` (Reference swaps `section: reference` and `ReferencePage`):

1. Add `content/new-chapter.md` with `title`, `section: learn`, and `status: shipped` or `not-yet`. Shipped `drac` fences must compile. Not-yet pages have no fences. Done when frontmatter parses and the body is the public markdown subset.
2. Add `src/routes/new-chapter.tsx` with `createFileRoute("/new-chapter")`, `loadMarkdownPage("new-chapter")`, and `<LearnPage page={page} />`. Add `{ path: "/new-chapter" }` to `vite.config.ts` `prerender.pages`. Wire LearnNav, LearnHubCards, `learnSequence.ts`, and `content/learn.md`. Done when the route, prerender path, and hub copy all name the slug.
3. Extend `src/tests/learn-pages.test.ts` and the hub or pager tests that cover the sequence. Run `pnpm --dir ../draconic-web exec vitest run learn-pages` and `pnpm --dir ../draconic-web typecheck`. Done when those pass.

## Run

- **dev**: `pnpm --dir ../draconic-web dev`
- **build**: `pnpm --dir ../draconic-web build`. Language-repo `scripts/generate-website.sh` stages GitHub Pages from that build.
- **typecheck**: `pnpm --dir ../draconic-web typecheck`

File target ≤1000 LOC, hard 1250. Markdown never tables.

## Reach

- **frontend-development**: React, Tailwind v4, CVA, Start file routes, nested `src/`
- **spec**: `docs/specs/draconic/public-site/`
- **docs**: durable notes. Public teaching stays in `draconic-web/content/`. Agent notes stay in `docs/`.
- **management**: working tracker
