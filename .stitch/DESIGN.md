# Design System: Draconic

**Project ID:** 2818161481079012082

## 1. Visual Theme & Atmosphere

Public Learn and Reference site for the Draconic language. Light, editorial documentation in the register of react.dev and the TypeScript handbook: paper-and-ink, generous whitespace, a reading column rather than an app shell. Not a dashboard, not SaaS marketing, no stock photos, no flashy gradients. Hierarchy comes from size and weight. Code is well-lit when a page is shipped.

## 2. Color Palette & Roles

- **Page Canvas (#F7F5F0)**: document background
- **Surface Paper (#FBF9F4)**: header bar and raised reading surface
- **Ink Primary (#1B1C19)**: wordmark, titles, body
- **Ink Secondary (#57534E)**: ledes, inactive nav, meta, not-yet badge text
- **Nav Mute (#625E59)**: unselected primary-nav labels
- **Hairline (#E7E5E4)**: header rule, section rules, chapter row dividers
- **Ember Accent (#9A3412)**: text links, path titles, small marks
- **Ember Strong (#842503)**: current-nav underline and hover
- **Badge Shipped fill (#ECFDF3)**: shipped pill background
- **Badge Shipped text (#166534)**: shipped pill label
- **Badge Not-yet fill (#F5F5F4)**: not-yet pill background
- **Code Well (#F4F1EA)**: shipped code fences only
- **Code Ink (#1B1C19)**: code text on the well

## 3. Typography Rules

- **Wordmark and display titles**: Source Serif 4. Wordmark at headline-md (24px / 32px, weight 600). Page titles at display (48px / 56px, weight 700, tracking -0.02em).
- **Section headings**: Source Serif 4 at headline-md (24px / 32px, weight 600) or headline-lg (32px / 40px) for article H1.
- **Body**: Source Sans 3. Lede 18px / 30px. Body 16px / 26px. Color is Ink Primary or Ink Secondary, never accent for paragraphs.
- **Nav and badges**: IBM Plex Sans 14px / 20px, weight 500. Current Learn or Reference is weight 700 with a 2px Ember Strong underline.
- **Code**: JetBrains Mono 14px / 24px inside Code Well. No code on not-yet pages.

## 4. Component Stylings

- **Chrome**: text wordmark "Draconic" left; primary nav Learn | Reference | GitHub right; 64px header on Surface Paper with a Hairline bottom edge; content aligned to an 820px column on landing, or header full-bleed with 2rem gutter on chapter pages. No search, no theme toggle, no filled CTA.
- **Tables / lists**: never data tables. Chapter lists and CLI commands are vertical rows with Hairline separators, name plus one-line description, optional badge.
- **Badges / status**: compact uppercase pills. **shipped** uses Badge Shipped fill and text. **not-yet** uses Badge Not-yet fill and Ink Secondary. One status badge on every page title row.
- **Sidebar (Learn and Reference working pages)**: 280px left nav, Hairline on the right edge. Current item Ember Accent; others Ink Secondary. Learn items: Install, from JavaScript, from systems, Dual worlds, modules, native types, host I/O, packages. Reference items: CLI, types, Dual-world rules, host I/O, packages.
- **Code fence**: rounded 2px, Code Well fill, JetBrains Mono, used only on shipped pages.

## 5. Layout Principles

Desktop web at 1440px. One focused reading surface. Landing is a centered 820px column: title, pitch, two-path chooser, chapter list. Chapter and Reference pages are a 280px sidebar plus an article column. Touch-friendly nav (~40px+ hits). Footer is a Hairline then the wordmark and a GitHub text link.
