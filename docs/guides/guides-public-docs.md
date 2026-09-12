---
id: guides-public-docs
title: Public Learn and Reference versus this vault
kind: guide
description: How website/ markdown remains the public Learn and Reference source, how TanStack Start presents it, and why agent docs stay in docs/.
domain: draconic
area: guides
tags: [guide, public-docs, website]
created_at: "2026-09-06"
updated_at: "2026-09-12"
---

# Public Learn and Reference versus this vault

## Overview

Public Learn and Reference remain sourced from `website/` markdown (`title`, `section`, `status`). That tree is not this vault ([[0010-public-docs-draconic-ssg]], [[specs/draconic/public-site/purpose|Public site purpose]]). Presentation is a TanStack Start app under `website/` ([[0013-public-site-tanstack-start]]), which supersedes [[0010-public-docs-draconic-ssg]] only as the HTML renderer. Kept from 0010: Learn and Reference IA, shipped versus not-yet, fence compile, playground later, and the vault is not the site. Home is a language homepage, not Learn copied to index. Do not treat generated HTML as truth. Agent and Toolchain notes stay under `docs/`. Orientation: [[overview-toolchain]], [[overview-vault]].

## Prerequisites

- **Read `website/` as source**: `.md` files with `title`, `section`, and `status` frontmatter. Do not treat generated `.html` as the source of truth.
- **Know the split**: this vault ([[standards-docs-vault]]) is for agents and maintainers. Learn and Reference are for people writing Programs.
- **Publisher**: `scripts/generate-website.sh` wraps the TanStack Start static build (`pnpm --dir website build`) and stages HTML for GitHub Pages.

## Steps

1. **Edit markdown in `website/`**, not vault notes, when the change is public language teaching or working reference.

2. **Stay inside the subset**: headings, paragraphs, lists, code fences, links, and status frontmatter (`title`, `section`, `status`). Keep those tags.

3. **Tag each page `shipped` or `not-yet`**. Copy-paste code exists only on shipped pages and must build. CI extracts those fences and compiles them. Fence tests live in `tests/integration/tests/website_pipeline.rs`. Playground and in-page runners are later ([[0010-public-docs-draconic-ssg]]).

4. **Do not treat generated HTML as truth.** The TanStack Start static build is the publisher. Home landing is a language homepage, not Learn copied to `index.html`.

5. **Publish path**: GitHub Pages hosts the public site. Dist is not committed as source.

6. **Keep agent docs in `docs/`**. CLI internals, Loop, NFRs, and Embed APIs belong here ([[api-cli]], [[api-embed]], [[guides-loop]], [[guides-toolchain]]). Do not serve the vault as the site.

## Examples

Learn path (from `website/learn.md`): Install, then from JavaScript or from systems, joining at Dual worlds, then modules, native types, host I/O, packages.

Reference path (from `website/reference.md`): CLI, types, Dual-world rules, host I/O, packages. Working pages while writing a Program, not a generated API dump. The vault API notes ([[api-cli]], [[api-embed]]) are the command and crate contracts for agents.

Status in `website/*.md` as of this note:

- **shipped**: Learn, Reference, Install, from JavaScript, from systems, Dual worlds, CLI
- **not-yet**: modules, native types, host I/O, packages, types, Dual-world rules, Reference host I/O, Reference packages

Shipped pages may include copy-paste fences. Not-yet pages stay prose.

Incorrect:

- **Vault as site**: publishing `docs/` to GitHub Pages
- **VitePress, Starlight, mdBook, or Next.js**: replacing the TanStack Start app
- **Copy-paste on not-yet**: a fence on a `status: not-yet` page
- **Documenting samples that do not compile** on shipped pages

## Reference

- **Split kept**: [[0010-public-docs-draconic-ssg]]
- **Presentation**: [[0013-public-site-tanstack-start]]
- **Public site purpose**: [[specs/draconic/public-site/purpose|Public site purpose]]
- **Publisher**: `scripts/generate-website.sh` wrapping `pnpm --dir website build`
- **Public URL**: GitHub Pages for `hembrow-innovations/draconic`
- **Developer CLI flow**: [[guides-toolchain]]
- **Vault layout**: [[standards-docs-vault]]
- **Leftover vault guides**: `docs/reference/guides/` ([[issue-tracker]], [[triage-labels]]) are not the public site and are not this guide
