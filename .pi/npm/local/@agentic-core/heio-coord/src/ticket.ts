import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export type TicketResult = { ok: boolean; text: string };

const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

function nextTicketNumber(cwd: string): number {
	const root = join(cwd, ".heio", "tickets");
	if (!existsSync(root)) return 1;
	let max = 0;
	for (const name of readdirSync(root)) {
		const match = name.match(/^ticket-(\d+)-/);
		if (!match?.[1]) continue;
		max = Math.max(max, Number(match[1]));
	}
	return max + 1;
}

function ticketMarkdown(input: {
	id: string;
	title: string;
	now: string;
}): string {
	return `---
id: "${input.id}"
title: "${input.title}"
kind: ticket
status: open
labels: feature
tags: []
created_at: "${input.now}"
updated_at: "${input.now}"
---

# ${input.title}

## Signal

What arrived. Bug, complaint, request, idea. Not yet work.

## Fit

Unknown until triage. Then one of:

- this slice → promote to a task
- this project, later slice → park
- changes the bet → escalate

## Notes

Facts only. The solution lives on a slice spec, not here.
`;
}

export function createTicket(input: {
	cwd: string;
	slug: string;
	now?: string;
}): TicketResult {
	if (!SLUG.test(input.slug)) {
		return {
			ok: false,
			text: `Use heio_stack. Invalid ticket slug: ${input.slug}`,
		};
	}
	const n = String(nextTicketNumber(input.cwd)).padStart(2, "0");
	const id = `ticket-${n}-${input.slug}`;
	const dir = join(input.cwd, ".heio", "tickets");
	mkdirSync(dir, { recursive: true });
	const path = join(dir, `${id}.md`);
	if (existsSync(path)) {
		return { ok: false, text: `Use heio_stack. ${id} already exists.` };
	}
	const now = input.now ?? new Date().toISOString();
	writeFileSync(path, ticketMarkdown({ id, title: input.slug, now }), "utf8");
	return { ok: true, text: `wrote ${id}` };
}
