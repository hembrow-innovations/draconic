---
id: "ticket-834-install-zed-editor"
title: "Install never taught the Zed highlighter"
kind: ticket
status: closed
ticket_type: observation
tags: [website, public-site]
blocked_by: []
created_at: "2026-09-12T13:20:00Z"
updated_at: "2026-09-12T13:23:00Z"
---

# Install never taught the Zed highlighter

## Signal

Closed. Website swarm shipped `website/install.md` with a Zed editor heading after the native hello build, a GitHub `editors/zed` link, and `draconic check` plus `draconic fmt` as the write loop. Search for Zed and editor hits `/install#zed-editor`. No language server, no VS Code tree, no new Learn chapter.

## Fit

In scope of `public-site.ia:learn-walkable`. Editor chrome is not a new nav item.

## Notes

- Page: `website/install.md`
- Locks: `website/src/tests/learn-pages.test.ts`, `website/src/tests/markdown-render.test.ts`, `website/src/tests/search.test.ts`
- Extension: `editors/zed`
- Do not claim an LSP process

## Parent

[[Public site — Contract]]
