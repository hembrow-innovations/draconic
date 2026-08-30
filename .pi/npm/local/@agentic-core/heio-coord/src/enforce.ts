import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";
import type {
	ExtensionContext,
	ToolCallEvent,
	ToolCallEventResult,
} from "@earendil-works/pi-coding-agent";

export const STICKY_REASON = "Use heio_stack. That path is sticky planning.";
export const EXPECT_REASON = "Use heio_stack. EXPECT is frozen.";
export const TASKS_REASON = "Use heio_stack. Slice must be frozen or active.";
export const ACTIVE_REASON = "Use heio_stack. Only one slice may be active.";

const UNICODE_SPACES = /[\u00A0\u2000-\u200A\u202F\u205F\u3000]/g;

function toolPath(input: unknown): string | undefined {
	if (!input || typeof input !== "object" || !("path" in input)) {
		return undefined;
	}
	const path = input.path;
	return typeof path === "string" ? path : undefined;
}

function toolCommand(input: unknown): string | undefined {
	if (!input || typeof input !== "object" || !("command" in input)) {
		return undefined;
	}
	const command = input.command;
	return typeof command === "string" ? command : undefined;
}

function stripQuotes(token: string): string {
	if (
		(token.startsWith('"') && token.endsWith('"') && token.length >= 2) ||
		(token.startsWith("'") && token.endsWith("'") && token.length >= 2)
	) {
		return token.slice(1, -1);
	}
	return token;
}

function bashMutatesPath(
	cwd: string,
	command: string,
	match: (cwd: string, rawPath: string) => boolean,
): boolean {
	const redirect = /(?:>>|>)\s*([^\s;|&]+)/g;
	for (const found of command.matchAll(redirect)) {
		const target = stripQuotes(found[1] ?? "");
		if (target && match(cwd, target)) return true;
	}
	if (!/\b(?:tee|rm|mv|cp)\b/.test(command)) return false;
	for (const raw of command.split(/[\s;|&]+/)) {
		const token = stripQuotes(raw);
		if (
			!token ||
			token === "tee" ||
			token === "rm" ||
			token === "mv" ||
			token === "cp"
		) {
			continue;
		}
		if (match(cwd, token)) return true;
	}
	return false;
}

function toolContent(input: unknown): string | undefined {
	if (!input || typeof input !== "object" || !("content" in input)) {
		return undefined;
	}
	const content = input.content;
	return typeof content === "string" ? content : undefined;
}

function resolveToolPath(cwd: string, rawPath: string): string {
	let normalized = rawPath.replace(UNICODE_SPACES, " ");
	if (normalized.startsWith("@")) normalized = normalized.slice(1);
	if (normalized === "~") normalized = homedir();
	else if (normalized.startsWith("~/")) {
		normalized = joinHome(normalized.slice(2));
	}
	if (/^file:\/\//.test(normalized)) {
		normalized = fileURLToPath(normalized);
	}
	return resolve(cwd, normalized);
}

function joinHome(rest: string): string {
	return resolve(homedir(), rest);
}

function isStickyPlanningPath(cwd: string, rawPath: string): boolean {
	const absolute = resolveToolPath(cwd, rawPath);
	const root = resolve(cwd, ".heio", "planning");
	const rel = relative(root, absolute);
	if (!rel || rel.startsWith("..") || rel === ".") return false;
	if (rel === "intent.md" || rel === "roadmap.md") return true;
	const parts = rel.split(sep);
	return parts.length === 3 && parts[0] === "sprints" && parts[2] === "shape.md";
}

function isOraclePath(cwd: string, rawPath: string): boolean {
	const absolute = resolveToolPath(cwd, rawPath);
	if (absolute.split(sep).pop() !== "oracles.md") return false;
	const rel = relative(resolve(cwd, ".heio"), absolute);
	return Boolean(rel) && rel !== "." && !rel.startsWith("..");
}

function expectLines(text: string): string[] {
	return text.split("\n").filter((line) => /^\s*EXPECT:\s*/.test(line));
}

function expectChanged(oldText: string, newText: string): boolean {
	const before = expectLines(oldText).join("\n");
	const after = expectLines(newText).join("\n");
	return before !== after;
}

function editSnippets(
	input: unknown,
): Array<{ oldText: string; newText: string }> {
	if (!input || typeof input !== "object") return [];
	const obj = input as Record<string, unknown>;
	if (typeof obj.oldText === "string" || typeof obj.newText === "string") {
		return [
			{
				oldText: typeof obj.oldText === "string" ? obj.oldText : "",
				newText: typeof obj.newText === "string" ? obj.newText : "",
			},
		];
	}
	if (!Array.isArray(obj.edits)) return [];
	const snippets: Array<{ oldText: string; newText: string }> = [];
	for (const edit of obj.edits) {
		if (!edit || typeof edit !== "object") continue;
		const item = edit as Record<string, unknown>;
		snippets.push({
			oldText: typeof item.oldText === "string" ? item.oldText : "",
			newText: typeof item.newText === "string" ? item.newText : "",
		});
	}
	return snippets;
}

function editPatchesExpect(input: unknown): boolean {
	for (const snippet of editSnippets(input)) {
		if (expectChanged(snippet.oldText, snippet.newText)) return true;
	}
	return false;
}

function writePatchesExpect(
	cwd: string,
	rawPath: string,
	content: string,
): boolean {
	if (!isOraclePath(cwd, rawPath)) return false;
	let existing = "";
	try {
		existing = readFileSync(resolveToolPath(cwd, rawPath), "utf8");
	} catch {
		return false;
	}
	return expectChanged(existing, content);
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
		let value = trimmed.slice("status:".length).trim();
		if (
			(value.startsWith('"') && value.endsWith('"') && value.length >= 2) ||
			(value.startsWith("'") && value.endsWith("'") && value.length >= 2)
		) {
			value = value.slice(1, -1);
		}
		return value;
	}
	return undefined;
}

function isSliceTasksPath(cwd: string, rawPath: string): boolean {
	const absolute = resolveToolPath(cwd, rawPath);
	if (absolute.split(sep).pop() !== "tasks.md") return false;
	const rel = relative(resolve(cwd, ".heio", "planning", "sprints"), absolute);
	if (!rel || rel.startsWith("..")) return false;
	const parts = rel.split(sep);
	return parts.length === 4 && parts[1] === "slices";
}

function tasksWriteBlocked(cwd: string, rawPath: string): boolean {
	if (!isSliceTasksPath(cwd, rawPath)) return false;
	const status = readStatus(
		join(dirname(resolveToolPath(cwd, rawPath)), "spec.md"),
	);
	return status !== "frozen" && status !== "active";
}

function listDirs(path: string): string[] {
	if (!existsSync(path)) return [];
	return readdirSync(path, { withFileTypes: true })
		.filter((ent) => ent.isDirectory())
		.map((ent) => ent.name);
}

function isSliceSpecPath(cwd: string, rawPath: string): boolean {
	const absolute = resolveToolPath(cwd, rawPath);
	if (absolute.split(sep).pop() !== "spec.md") return false;
	const rel = relative(resolve(cwd, ".heio", "planning", "sprints"), absolute);
	if (!rel || rel.startsWith("..")) return false;
	const parts = rel.split(sep);
	return parts.length === 4 && parts[1] === "slices";
}

function statusIn(text: string): string | undefined {
	for (const line of text.split("\n")) {
		const trimmed = line.trim();
		if (!trimmed.startsWith("status:")) continue;
		let value = trimmed.slice("status:".length).trim();
		if (
			(value.startsWith('"') && value.endsWith('"') && value.length >= 2) ||
			(value.startsWith("'") && value.endsWith("'") && value.length >= 2)
		) {
			value = value.slice(1, -1);
		}
		return value;
	}
	return undefined;
}

function setsActive(event: ToolCallEvent): boolean {
	if (event.toolName === "edit") {
		for (const snippet of editSnippets(event.input)) {
			if (statusIn(snippet.newText) === "active") return true;
		}
		return false;
	}
	if (event.toolName === "write") {
		const content = toolContent(event.input);
		return content !== undefined && statusIn(content) === "active";
	}
	return false;
}

function anotherSliceIsActive(cwd: string, thisSpec: string): boolean {
	const sprintsRoot = resolve(cwd, ".heio", "planning", "sprints");
	for (const sprint of listDirs(sprintsRoot)) {
		const slicesRoot = join(sprintsRoot, sprint, "slices");
		for (const slug of listDirs(slicesRoot)) {
			const spec = join(slicesRoot, slug, "spec.md");
			if (resolve(spec) === resolve(thisSpec)) continue;
			if (readStatus(spec) === "active") return true;
		}
	}
	return false;
}

function secondActiveBlocked(
	cwd: string,
	event: ToolCallEvent,
	rawPath: string,
): boolean {
	if (!isSliceSpecPath(cwd, rawPath)) return false;
	if (!setsActive(event)) return false;
	const absolute = resolveToolPath(cwd, rawPath);
	if (readStatus(absolute) === "active") return false;
	return anotherSliceIsActive(cwd, absolute);
}

export function blockIllegalWrite(
	event: ToolCallEvent,
	ctx: Pick<ExtensionContext, "cwd">,
): ToolCallEventResult | undefined {
	if (event.toolName === "bash") {
		const command = toolCommand(event.input);
		if (!command) return undefined;
		if (bashMutatesPath(ctx.cwd, command, isStickyPlanningPath)) {
			return { block: true, reason: STICKY_REASON };
		}
		if (bashMutatesPath(ctx.cwd, command, isOraclePath)) {
			return { block: true, reason: EXPECT_REASON };
		}
		if (bashMutatesPath(ctx.cwd, command, tasksWriteBlocked)) {
			return { block: true, reason: TASKS_REASON };
		}
		return undefined;
	}
	if (event.toolName !== "write" && event.toolName !== "edit") return undefined;
	const path = toolPath(event.input);
	if (!path) return undefined;
	if (isStickyPlanningPath(ctx.cwd, path)) {
		return { block: true, reason: STICKY_REASON };
	}
	if (tasksWriteBlocked(ctx.cwd, path)) {
		return { block: true, reason: TASKS_REASON };
	}
	if (secondActiveBlocked(ctx.cwd, event, path)) {
		return { block: true, reason: ACTIVE_REASON };
	}
	if (
		event.toolName === "edit" &&
		isOraclePath(ctx.cwd, path) &&
		editPatchesExpect(event.input)
	) {
		return { block: true, reason: EXPECT_REASON };
	}
	if (event.toolName === "write") {
		const content = toolContent(event.input);
		if (content !== undefined && writePatchesExpect(ctx.cwd, path, content)) {
			return { block: true, reason: EXPECT_REASON };
		}
	}
	return undefined;
}
