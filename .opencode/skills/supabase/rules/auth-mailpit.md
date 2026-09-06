---
title: Read local mail in Mailpit
impact: MEDIUM
impactDescription: confirm links never hit a real inbox locally
tags: [auth]
---

## Read local mail in Mailpit

Local Auth sends mail to Mailpit (default `http://127.0.0.1:54324`). There is no real SMTP unless you configured one.

**Incorrect:** Waiting on Gmail for a local confirm link, or wiring production SMTP into `config.toml` for everyday dev.

**Correct:** Open the Mailpit URL from `supabase status`. Click the confirm or magic link there.

Notes: Older docs said Inbucket. Current CLI output says Mailpit on the same default port.
