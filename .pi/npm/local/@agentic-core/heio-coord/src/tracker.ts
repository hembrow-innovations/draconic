import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

export type NamedTracker = {
	file: "AGENTS.md" | "WORKSPACE.md";
	name: string;
};

const TRACKER_FILES = ["AGENTS.md", "WORKSPACE.md"] as const;
const RUNS_TRACKER = /runs\s+\*\*([^*]+)\*\*/i;

function trackerSection(markdown: string): string | undefined {
	const match = markdown.match(/^##\s+Tracker\s*$/m);
	if (!match || match.index === undefined) return undefined;
	const start = match.index + match[0].length;
	const rest = markdown.slice(start);
	const next = rest.search(/^##\s+/m);
	return (next === -1 ? rest : rest.slice(0, next)).trim();
}

function trackerName(markdown: string): string | undefined {
	const section = trackerSection(markdown);
	const haystack = section && section.length > 0 ? section : markdown;
	const match = haystack.match(RUNS_TRACKER);
	const name = match?.[1]?.trim();
	return name && name.length > 0 ? name : undefined;
}

export function namedTracker(cwd: string): NamedTracker | null {
	for (const file of TRACKER_FILES) {
		const path = join(cwd, file);
		if (!existsSync(path)) continue;
		let raw: string;
		try {
			raw = readFileSync(path, "utf8");
		} catch {
			continue;
		}
		const name = trackerName(raw);
		if (name) return { file, name };
	}
	return null;
}

export function inertReason(named: NamedTracker): string {
	return `${named.file} names ${named.name}. Coordinator stays inert.`;
}

export function isForeignTracker(
	named: NamedTracker | null,
): named is NamedTracker {
	return named !== null && named.name !== "heio-stack";
}
