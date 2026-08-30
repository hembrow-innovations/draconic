import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { readStackStatus } from "./status.ts";

export const CLAIM_TTL_MS = 30 * 60 * 1000;

export type ClaimResult = { ok: boolean; text: string };

type TaskBlock = {
	id: string;
	start: number;
	end: number;
	claim?: string;
	claimedAt?: string;
};

function activeTasksPath(cwd: string): string | undefined {
	const status = readStackStatus(cwd);
	if (status.freeze !== "active" || !status.sprintId || !status.sliceId) {
		return undefined;
	}
	return join(
		cwd,
		".heio",
		"planning",
		"sprints",
		status.sprintId,
		"slices",
		status.sliceId,
		"tasks.md",
	);
}

function parseTaskBlocks(markdown: string): TaskBlock[] {
	const lines = markdown.split("\n");
	const blocks: TaskBlock[] = [];
	let current:
		| { id: string; start: number; claim?: string; claimedAt?: string }
		| undefined;
	const flush = (end: number) => {
		if (!current) return;
		blocks.push({ ...current, end });
		current = undefined;
	};
	for (let i = 0; i < lines.length; i++) {
		const line = lines[i] ?? "";
		const head = line.match(/^- \[[ x]\] (T\d+):/);
		if (head?.[1]) {
			flush(i);
			current = { id: head[1], start: i };
			continue;
		}
		if (!current) continue;
		const claim = line.match(/^\s*claim:\s*(.+)\s*$/);
		if (claim?.[1]) current.claim = claim[1].replace(/^["']|["']$/g, "");
		const claimedAt = line.match(/^\s*claimed_at:\s*(.+)\s*$/);
		if (claimedAt?.[1])
			current.claimedAt = claimedAt[1].replace(/^["']|["']$/g, "");
	}
	flush(lines.length);
	return blocks;
}

function isLiveClaim(block: TaskBlock, now: number): string | undefined {
	if (!block.claim) return undefined;
	if (!block.claimedAt) return block.claim;
	const at = Date.parse(block.claimedAt);
	if (Number.isNaN(at) || now - at > CLAIM_TTL_MS) return undefined;
	return block.claim;
}

function upsertClaimLines(
	lines: string[],
	block: TaskBlock,
	sessionId: string,
	at: string,
): string[] {
	const next = lines.slice();
	const kept: string[] = [];
	for (let i = block.start; i < block.end; i++) {
		const line = next[i] ?? "";
		if (/^\s*claim:/.test(line) || /^\s*claimed_at:/.test(line)) continue;
		kept.push(line);
	}
	const head = kept[0] ?? "";
	const rest = kept.slice(1);
	const rebuilt = [
		head,
		`  claim: ${sessionId}`,
		`  claimed_at: ${at}`,
		...rest,
	];
	next.splice(block.start, block.end - block.start, ...rebuilt);
	return next;
}

function isTaskId(target: string): boolean {
	return /^T\d+$/.test(target);
}

function unquote(value: string): string {
	const trimmed = value.trim();
	if (
		(trimmed.startsWith('"') && trimmed.endsWith('"') && trimmed.length >= 2) ||
		(trimmed.startsWith("'") && trimmed.endsWith("'") && trimmed.length >= 2)
	) {
		return trimmed.slice(1, -1);
	}
	return trimmed;
}

function readField(path: string, key: string): string | undefined {
	let raw: string;
	try {
		raw = readFileSync(path, "utf8");
	} catch {
		return undefined;
	}
	if (!raw.startsWith("---")) return undefined;
	const end = raw.indexOf("\n---", 3);
	if (end === -1) return undefined;
	for (const line of raw.slice(3, end).split("\n")) {
		const trimmed = line.trim();
		if (!trimmed.startsWith(`${key}:`)) continue;
		return unquote(trimmed.slice(key.length + 1));
	}
	return undefined;
}

function ticketPath(cwd: string, id: string): string | undefined {
	const root = join(cwd, ".heio", "tickets");
	if (!existsSync(root)) return undefined;
	const direct = join(root, `${id}.md`);
	if (existsSync(direct)) return direct;
	for (const name of readdirSync(root)) {
		if (!name.endsWith(".md")) continue;
		const path = join(root, name);
		if (readField(path, "id") === id) return path;
	}
	return undefined;
}

function upsertFrontmatter(
	raw: string,
	patch: Record<string, string | undefined>,
): string {
	if (!raw.startsWith("---")) return raw;
	const end = raw.indexOf("\n---", 3);
	if (end === -1) return raw;
	const body = raw.slice(end + 4);
	const lines = raw
		.slice(3, end)
		.split("\n")
		.filter((line) => {
			const trimmed = line.trim();
			const colon = trimmed.indexOf(":");
			if (colon <= 0) return true;
			const key = trimmed.slice(0, colon);
			return !(key in patch);
		});
	for (const [key, value] of Object.entries(patch)) {
		if (value === undefined) continue;
		lines.push(`${key}: "${value}"`);
	}
	return `---${lines.join("\n")}\n---${body.startsWith("\n") ? body : `\n${body}`}`;
}

function liveHolder(
	claim: string | undefined,
	claimedAt: string | undefined,
	now: number,
): string | undefined {
	return isLiveClaim({ id: "", start: 0, end: 0, claim, claimedAt }, now);
}

function claimTicket(input: {
	cwd: string;
	sessionId: string;
	target: string;
	now?: number;
}): ClaimResult {
	const path = ticketPath(input.cwd, input.target);
	if (!path) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is not a ticket.`,
		};
	}
	const now = input.now ?? Date.now();
	const holder = liveHolder(
		readField(path, "claim"),
		readField(path, "claimed_at"),
		now,
	);
	if (holder && holder !== input.sessionId) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is claimed by ${holder}.`,
		};
	}
	let raw: string;
	try {
		raw = readFileSync(path, "utf8");
	} catch {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is not a ticket.`,
		};
	}
	writeFileSync(
		path,
		upsertFrontmatter(raw, {
			claim: input.sessionId,
			claimed_at: new Date(now).toISOString(),
		}),
		"utf8",
	);
	return { ok: true, text: `claimed ${input.target}` };
}

function releaseTicket(input: {
	cwd: string;
	sessionId: string;
	target: string;
	now?: number;
}): ClaimResult {
	const path = ticketPath(input.cwd, input.target);
	if (!path) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is not a ticket.`,
		};
	}
	const now = input.now ?? Date.now();
	const holder = liveHolder(
		readField(path, "claim"),
		readField(path, "claimed_at"),
		now,
	);
	if (holder && holder !== input.sessionId) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is claimed by ${holder}.`,
		};
	}
	let raw: string;
	try {
		raw = readFileSync(path, "utf8");
	} catch {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is not a ticket.`,
		};
	}
	writeFileSync(
		path,
		upsertFrontmatter(raw, { claim: undefined, claimed_at: undefined }),
		"utf8",
	);
	return { ok: true, text: `released ${input.target}` };
}

function claimSliceTask(input: {
	cwd: string;
	sessionId: string;
	target: string;
	now?: number;
}): ClaimResult {
	const tasksPath = activeTasksPath(input.cwd);
	if (!tasksPath || !existsSync(tasksPath)) {
		return { ok: false, text: "Use heio_stack. Slice must be active." };
	}
	const now = input.now ?? Date.now();
	let markdown: string;
	try {
		markdown = readFileSync(tasksPath, "utf8");
	} catch {
		return { ok: false, text: "Use heio_stack. Slice must be active." };
	}
	const blocks = parseTaskBlocks(markdown);
	const block = blocks.find((item) => item.id === input.target);
	if (!block) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is not a slice task.`,
		};
	}
	const holder = isLiveClaim(block, now);
	if (holder && holder !== input.sessionId) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is claimed by ${holder}.`,
		};
	}
	const at = new Date(now).toISOString();
	const lines = markdown.endsWith("\n")
		? markdown.slice(0, -1).split("\n")
		: markdown.split("\n");
	const next = upsertClaimLines(lines, block, input.sessionId, at);
	writeFileSync(tasksPath, `${next.join("\n")}\n`, "utf8");
	return { ok: true, text: `claimed ${input.target}` };
}

function stripClaimLines(lines: string[], block: TaskBlock): string[] {
	const next = lines.slice();
	const kept: string[] = [];
	for (let i = block.start; i < block.end; i++) {
		const line = next[i] ?? "";
		if (/^\s*claim:/.test(line) || /^\s*claimed_at:/.test(line)) continue;
		kept.push(line);
	}
	next.splice(block.start, block.end - block.start, ...kept);
	return next;
}

function releaseSliceTask(input: {
	cwd: string;
	sessionId: string;
	target: string;
	now?: number;
}): ClaimResult {
	const tasksPath = activeTasksPath(input.cwd);
	if (!tasksPath || !existsSync(tasksPath)) {
		return { ok: false, text: "Use heio_stack. Slice must be active." };
	}
	let markdown: string;
	try {
		markdown = readFileSync(tasksPath, "utf8");
	} catch {
		return { ok: false, text: "Use heio_stack. Slice must be active." };
	}
	const now = input.now ?? Date.now();
	const blocks = parseTaskBlocks(markdown);
	const block = blocks.find((item) => item.id === input.target);
	if (!block) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is not a slice task.`,
		};
	}
	const holder = isLiveClaim(block, now);
	if (holder && holder !== input.sessionId) {
		return {
			ok: false,
			text: `Use heio_stack. ${input.target} is claimed by ${holder}.`,
		};
	}
	const lines = markdown.endsWith("\n")
		? markdown.slice(0, -1).split("\n")
		: markdown.split("\n");
	const next = stripClaimLines(lines, block);
	writeFileSync(tasksPath, `${next.join("\n")}\n`, "utf8");
	return { ok: true, text: `released ${input.target}` };
}

export function claimTask(input: {
	cwd: string;
	sessionId: string;
	target: string;
	now?: number;
}): ClaimResult {
	return isTaskId(input.target) ? claimSliceTask(input) : claimTicket(input);
}

export function releaseTask(input: {
	cwd: string;
	sessionId: string;
	target: string;
	now?: number;
}): ClaimResult {
	return isTaskId(input.target) ? releaseSliceTask(input) : releaseTicket(input);
}
