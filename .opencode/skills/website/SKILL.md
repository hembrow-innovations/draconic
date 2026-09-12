---
name: website
description: Website public Learn and Reference under website/. Use when editing website markdown, Learn, Reference, shipped or not-yet fences, site chrome, generate-website, or the docs site. When another skill needs this tree. React/Tailwind/CVA: load frontend-development. Not the docs/ vault.
---

# Website

Public language site. Teaching copy lives in `website/content/`. The Start app lives in nested `website/src/`. Load **frontend-development** for React, Tailwind v4, CVA, and Start craft. This tree owns local primitives under `website/src/components` and `website/src/features`. There is no `ui-components-web` kit and no TanStack Query.

Purpose, IA, and fence rules stay in `docs/specs/draconic/public-site/`. Vault-versus-site is ADR 0010. Start presentation is ADR 0013. Load **spec** and **docs** for those; this skill owns the tree.

## Discover first

1. Find teaching markdown. Law is `website/content/<slug>.md` with `title`, `section`, and `status`. If `website/*.md` still sits at the package root, relocate in this sitting before other Website edits. Done when the Vite glob, Vitest catalog, and `website_pipeline` fence checker read only `website/content/`, and the package root has no teaching `.md`.
2. Copy a neighboring route and primitive under `website/src/`. Home at `/` is JSX. Learn and Reference chapters load markdown through `DocsShell`, `LearnPage`, or `ReferencePage`. Done when the new file sits beside its neighbor in the same folder shape (`Component.tsx`, `.types.ts`, `.variants.ts`, `index.ts`).
3. Load **frontend-development** for tokens, CVA, nested `src/`, and Start file routes. Hex lives in `website/src/styles/theme.css`. Done when the sitting uses those rules instead of inventing a second palette or kit.

## Tree

- **website/content/**: teaching markdown. One file per slug. Slug is the basename. Keep this folder flat so `loadMarkdownPage` can keep using the stem.
- **website/src/routes/**: file routes. `routeTree.gen.ts` is generated. Add the path to `prerender.pages` in `website/vite.config.ts` (`crawlLinks` is false).
- **website/src/components/** and **website/src/features/**: chrome and Learn/Reference/home UI.
- **website/src/lib/content/**: loader and renderer, not teaching copy. Point the glob at `../../../content/*.md`.
- **website/src/tests/**: Vitest source contracts. Run `pnpm --dir website exec vitest run <file-stem>`. The package `test` script runs search only.
- **website/ root**: `package.json`, `pnpm-lock.yaml`, `vite.config.ts`, `tsconfig.json`, `.gitignore`.

Live HTML is the Start prerender. Author markdown. Leftover `/website/*.html` is gitignored retired SSG output. Markdown may keep `install.html` style hrefs; `toAppHref` rewrites them.

Teaching copy stays out of `website/src/` so the loader module `website/src/lib/content/` does not share a folder with pages.

## Relocate

When teaching files are still `website/*.md`:

1. Move each teaching `.md` into `website/content/`. Move markdown only. Done when `website/content/` holds every former root teaching file and no teaching `.md` remains beside `package.json`.
2. Retarget `import.meta.glob` in `website/src/lib/content/loadMarkdown.ts` to `../../../content/*.md`. Point Vitest `websiteDir` joins and `readdirSync` catalog checks at `website/content`. Point `check_fences` in `tests/integration/tests/website_pipeline.rs` at `website/content` (a checker left on `website/` would skip every fence). Retarget Rust tests that read `website/install.md` or `website/cli.md`. Done when every path that loaded root `*.md` now loads `website/content/*.md`.
3. Verify with `pnpm --dir website typecheck`, `pnpm --dir website exec vitest run markdown-loader`, `pnpm --dir website exec vitest run search`, `pnpm --dir website exec vitest run learn-pages`, and the website pipeline integration test. Done when those pass and search still lists every slug.

## Add a page

For a Learn chapter `new-chapter` (Reference swaps `section: reference` and `ReferencePage`):

1. Add `website/content/new-chapter.md` with `title`, `section: learn`, and `status: shipped` or `not-yet`. Shipped `drac` fences must compile. Not-yet pages have no fences. Done when frontmatter parses and the body is the public markdown subset.
2. Add `website/src/routes/new-chapter.tsx` with `createFileRoute("/new-chapter")`, `loadMarkdownPage("new-chapter")`, and `<LearnPage page={page} />`. Add `{ path: "/new-chapter" }` to `website/vite.config.ts` `prerender.pages`. Wire LearnNav, LearnHubCards, `learnSequence.ts`, and `website/content/learn.md`. Done when the route, prerender path, and hub copy all name the slug.
3. Extend `website/src/tests/learn-pages.test.ts` and the hub or pager tests that cover the sequence. Run `pnpm --dir website exec vitest run learn-pages` and `pnpm --dir website typecheck`. Done when those pass.

## Run

- **dev**: `pnpm --dir website dev`
- **build**: `pnpm --dir website build`. `scripts/generate-website.sh` stages GitHub Pages.
- **typecheck**: `pnpm --dir website typecheck`

File target ≤1000 LOC, hard 1250. Markdown never tables.

## Reach

- **frontend-development**: React, Tailwind v4, CVA, Start file routes, nested `src/`
- **spec**: `docs/specs/draconic/public-site/`
- **docs**: durable notes. Public teaching stays in `website/content/`. Agent notes stay in `docs/`.
- **management**: working tracker
