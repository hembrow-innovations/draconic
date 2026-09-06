---
id: "website-redesign"
title: "Public site TanStack Start redesign"
kind: sprint
status: closed
tags: [ website, public-site ]
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T19:18:25Z"
---
# Public site TanStack Start redesign

## Grouping

Location [[location-589-public-site]]. Vertical cuts to replace the generate.drac dump with a TanStack Start app at TypeScript.org quality. One thin slice per sitting. Language ROADMAP.md loop is out.

## Slices in

- [[slice-590-scaffold-start-app]]: pnpm TypeScript TanStack Start app boots under website/. blocked_by: none
- [[slice-591-nested-src-root-route]]: nested src/ and `/` file route. blocked_by: [[slice-590-scaffold-start-app]]
- [[slice-592-semantic-tokens]]: Tailwind v4 @theme semantic tokens. blocked_by: [[slice-591-nested-src-root-route]]
- [[slice-593-typography-badge]]: type scale and shipped/not-yet badge CVA. blocked_by: [[slice-592-semantic-tokens]]
- [[slice-594-site-header-nav]]: wordmark and Learn / Reference / GitHub. blocked_by: [[slice-593-typography-badge]]
- [[slice-595-site-footer-skip]]: footer and skip-to-content. blocked_by: [[slice-594-site-header-nav]]
- [[slice-596-home-hero-cta]]: homepage hero and Get-started CTAs. blocked_by: [[slice-595-site-footer-skip]]
- [[slice-597-home-features]]: dual-backend feature grid. blocked_by: [[slice-596-home-hero-cta]]
- [[slice-598-markdown-loader]]: load website markdown frontmatter. blocked_by: [[slice-591-nested-src-root-route]]
- [[slice-599-markdown-render]]: render the markdown subset. blocked_by: [[slice-598-markdown-loader]]
- [[slice-600-docs-shell]]: article layout with sidebar and badge. blocked_by: [[slice-593-typography-badge]] [[slice-599-markdown-render]]
- [[slice-601-learn-hub-nav]]: Learn hub and chapter sidebar. blocked_by: [[slice-600-docs-shell]]
- [[slice-602-learn-pages]]: Learn chapter routes from existing md. blocked_by: [[slice-601-learn-hub-nav]]
- [[slice-603-learn-prev-next]]: prev/next along the Learn path. blocked_by: [[slice-602-learn-pages]]
- [[slice-604-reference-hub-pages]]: Reference hub and working pages. blocked_by: [[slice-600-docs-shell]]
- [[slice-605-search]]: find pages by title or heading. blocked_by: [[slice-602-learn-pages]] [[slice-604-reference-hub-pages]]
- [[slice-606-theme-toggle]]: light/dark via tokens. blocked_by: [[slice-592-semantic-tokens]]
- [[slice-607-mobile-a11y]]: small viewport nav and focus. blocked_by: [[slice-594-site-header-nav]]
- [[slice-608-fence-static-deploy]]: fences still honest; Start static build replaces generate.drac emit. blocked_by: [[slice-599-markdown-render]] [[slice-597-home-features]]

## Slices out

- playground
- in-page runners
- serving docs/ vault
- Next.js/VitePress/Starlight/mdBook
- language ROADMAP atoms
- rewriting ADR-0010’s vault split
- changing CONTEXT.md Learn/Reference terms
