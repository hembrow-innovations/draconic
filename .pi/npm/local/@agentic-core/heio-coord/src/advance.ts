import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { readStackStatus } from "./status.ts";

export type AdvanceResult = { ok: boolean; text: string };

const TARGETS = ["active", "met", "abandoned"] as const;
const ALLOWED: Record<string, readonly string[]> = {
	frozen: ["active"],
	active: ["met", "abandoned"],
};

export function isBuilderShaped(input: {
	prompt?: string;
	sessionName?: string;
	agent?: string;
}): boolean {
	const haystack = [input.prompt, input.sessionName, input.agent]
		.filter((value): value is string => typeof value === "string")
		.join("\n");
	return /\bheio-builder\b/.test(haystack);
}

function specPath(cwd: string): string | undefined {
	const status = readStackStatus(cwd);
	if (!status.sprintId || !status.sliceId) return undefined;
	return join(
		cwd,
		".heio",
		"planning",
		"sprints",
		status.sprintId,
		"slices",
		status.sliceId,
		"spec.md",
	);
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

function readStatus(path: string): string | undefined {
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
		if (!trimmed.startsWith("status:")) continue;
		return unquote(trimmed.slice("status:".length));
	}
	return undefined;
}

function setStatus(raw: string, status: string): string {
	if (!raw.startsWith("---")) return raw;
	const end = raw.indexOf("\n---", 3);
	if (end === -1) return raw;
	const head = raw.slice(0, end);
	const rest = raw.slice(end);
	if (/^status:\s*.*$/m.test(head)) {
		return `${head.replace(/^status:\s*.*$/m, `status: "${status}"`)}${rest}`;
	}
	return `${head}\nstatus: "${status}"${rest}`;
}

export function advanceSlice(input: {
	cwd: string;
	target: string;
	builder: boolean;
}): AdvanceResult {
	if (input.builder) {
		return {
			ok: false,
			text: "Use heio_stack. Builder cannot mark the slice met.",
		};
	}
	if (!TARGETS.includes(input.target as (typeof TARGETS)[number])) {
		return {
			ok: false,
			text: "target must be active, met, or abandoned",
		};
	}
	const path = specPath(input.cwd);
	if (!path || !existsSync(path)) {
		return { ok: false, text: "Use heio_stack. No slice to advance." };
	}
	const current = readStatus(path);
	const allowed = current ? ALLOWED[current] : undefined;
	if (!current || !allowed || !allowed.includes(input.target)) {
		return {
			ok: false,
			text: `Use heio_stack. Cannot advance ${current ?? "none"} to ${input.target}.`,
		};
	}
	let raw: string;
	try {
		raw = readFileSync(path, "utf8");
	} catch {
		return { ok: false, text: "Use heio_stack. No slice to advance." };
	}
	const status = readStackStatus(input.cwd);
	writeFileSync(path, setStatus(raw, input.target), "utf8");
	return {
		ok: true,
		text: `advanced ${status.sliceId} to ${input.target}`,
	};
}
