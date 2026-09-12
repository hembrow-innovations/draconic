---
id: "ticket-829-install-from-source"
title: "Install sent clone-build off-page while curl needs missing releases"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T12:20:00Z"
updated_at: "2026-09-12T12:30:00Z"
---

# Install sent clone-build off-page while curl needs missing releases

## Signal

Closed. Website swarm shipped `website/install.md` with a From source heading, clone-build `cargo build -p draconic-cli --release`, PATH to `target/release/draconic`, and a GitHub README link. Curl one-liner stays. `website/src/tests/learn-pages.test.ts`, `website/src/tests/markdown-render.test.ts`, and `website/src/tests/search.test.ts` lock that path. No PowerShell installer, no invented install URL, no GitHub Releases publish.

## Fit

In scope of `public-site.ia:learn-walkable` and the Get-started CTA in `public-site.home:landing`. Publishing release artifacts is D01 distribution, not this area.

## Notes

- Page: `website/install.md`
- Locks: `website/src/tests/learn-pages.test.ts`, `website/src/tests/markdown-render.test.ts`, `website/src/tests/search.test.ts`
- Script still fetches GitHub Releases: `scripts/install.sh`
- Do not invent an installer URL

## Parent

[[Public site — Contract]]
