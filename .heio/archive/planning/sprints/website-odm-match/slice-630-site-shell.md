---
id: "slice-630-site-shell"
title: "ODM site shell"
kind: slice
status: met
sprint: "website-odm-match"
blocked_by:
  - slice-629-odm-tokens
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T22:50:00Z"
---

# ODM site shell

## Why

ODM chrome is not a top bar. Every page is skip link, sticky side nav, then main. Wordmark, Learn, Reference, and GitHub live in that nav. Search and theme toggle stay, in the side, because those promises remain.

## Done

Every route, including `/`, renders skip-to-content, a sticky side nav with wordmark plus Learn, Reference, and GitHub, and a main column. There is no site-wide top header bar.

## Blocked by

- [[slice-629-odm-tokens]]: shell classes use tokens, not hex.

## Non-goals

- **Home kicker and path steps**: [[slice-631-home-odm-layout]]
- **Docs article chrome**: [[slice-632-docs-article-odm]]
- **Learn and Reference groups in the side nav**: [[slice-633-docs-nav-groups]]
- **Small-viewport wrap**: [[slice-635-mobile-odm-wrap]]
- **Playground, vault-as-site, Start replacement**

## Oracle checklist

- [x] O1: root layout is skip plus sticky side nav plus main; side nav has wordmark, Learn, Reference, GitHub
  CHECK: pnpm --dir website exec vitest run site-header-primary-nav site-footer-and-skip-link
  EXPECT: Test Files  2 passed
  EVIDENCE: `pnpm --dir website exec vitest run site-header-primary-nav site-footer-and-skip-link` → Test Files  2 passed (2); task-637 completed in archive

## Pool

- `[[task-637-site-shell]]`

## See also

[[location-589-public-site]] [[website-odm-match]] public-site.chrome:odm-shell public-site.chrome:primary-nav website/src/routes/__root.tsx
