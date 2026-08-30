import {
	existsSync,
	mkdirSync,
	readdirSync,
	readFileSync,
	realpathSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

export type SessionId = string & { readonly __brand: "SessionId" };

const SESSION_ID_PATTERN = /^[A-Za-z0-9._-]+$/;
const UNICODE_SPACES = /[\u00A0\u2000-\u200A\u202F\u205F\u3000]/g;

export const STUB_TODO_MARKDOWN = `# Session checklists

Do not write a playbook checklist here. Call \`heio_todo\`.
Each session owns \`.heio/sessions/<sessionId>/TODO.md\`.
Use action \`list\` to see sibling checklists.
`;

export function parseSessionId(raw: string): SessionId {
	if (raw === "." || raw === ".." || !SESSION_ID_PATTERN.test(raw)) {
		throw new Error(`invalid session id: ${raw}`);
	}
	return raw as SessionId;
}

export function stubTodoPath(cwd: string): string {
	return join(cwd, ".heio", "TODO.md");
}

export function sessionTodoPath(cwd: string, sessionId: SessionId): string {
	return join(cwd, ".heio", "sessions", sessionId, "TODO.md");
}

function withTrailingNewline(markdown: string): string {
	return markdown.endsWith("\n") ? markdown : `${markdown}\n`;
}

function writeStubIfNeeded(stubPath: string): void {
	if (existsSync(stubPath)) {
		try {
			if (readFileSync(stubPath, "utf8") === STUB_TODO_MARKDOWN) return;
		} catch {
			// restore below
		}
	}
	writeFileSync(stubPath, STUB_TODO_MARKDOWN, "utf8");
}

export function writeSessionChecklist(input: {
	cwd: string;
	sessionId: SessionId;
	markdown: string;
}): { sessionPath: string; stubPath: string } {
	const sessionPath = sessionTodoPath(input.cwd, input.sessionId);
	const stubPath = stubTodoPath(input.cwd);
	mkdirSync(join(input.cwd, ".heio", "sessions", input.sessionId), {
		recursive: true,
	});
	writeFileSync(sessionPath, withTrailingNewline(input.markdown), "utf8");
	writeStubIfNeeded(stubPath);
	return { sessionPath, stubPath };
}

function firstNonEmptyLine(text: string): string {
	for (const line of text.split("\n")) {
		const trimmed = line.trim();
		if (trimmed) return trimmed;
	}
	return "(empty)";
}

export function listSessionChecklists(cwd: string): Array<{
	sessionId: string;
	path: string;
	title: string;
}> {
	const sessionsDir = join(cwd, ".heio", "sessions");
	if (!existsSync(sessionsDir)) return [];
	const listed: Array<{ sessionId: string; path: string; title: string }> = [];
	for (const entry of readdirSync(sessionsDir)) {
		let sessionId: SessionId;
		try {
			sessionId = parseSessionId(entry);
		} catch {
			continue;
		}
		const path = sessionTodoPath(cwd, sessionId);
		if (!existsSync(path) || !statSync(path).isFile()) continue;
		listed.push({
			sessionId,
			path,
			title: firstNonEmptyLine(readFileSync(path, "utf8")),
		});
	}
	return listed.sort((a, b) => a.sessionId.localeCompare(b.sessionId));
}

function resolveTodoToolPath(cwd: string, rawPath: string): string {
	let normalized = rawPath.replace(UNICODE_SPACES, " ");
	if (normalized.startsWith("@")) normalized = normalized.slice(1);
	if (normalized === "~") normalized = homedir();
	else if (normalized.startsWith("~/")) {
		normalized = join(homedir(), normalized.slice(2));
	}
	if (/^file:\/\//.test(normalized)) {
		normalized = fileURLToPath(normalized);
	}
	return resolve(cwd, normalized);
}

function existingCanonical(path: string): string | undefined {
	try {
		return realpathSync(path);
	} catch {
		return undefined;
	}
}

function withExistingCanonical(path: string): string[] {
	const canonical = existingCanonical(path);
	return canonical && canonical !== path ? [path, canonical] : [path];
}

function isSessionTodoUnder(root: string, absolute: string): boolean {
	const prefix = root.endsWith(sep) ? root : `${root}${sep}`;
	if (!absolute.startsWith(prefix)) return false;
	const rel = absolute.slice(prefix.length);
	const parts = rel.split(sep);
	if (parts.length !== 2 || parts[1] !== "TODO.md") return false;
	try {
		parseSessionId(parts[0] ?? "");
		return true;
	} catch {
		return false;
	}
}

function matchesProtectedShape(cwd: string, absolute: string): boolean {
	const stubPaths = withExistingCanonical(resolve(stubTodoPath(cwd)));
	const sessionRoots = withExistingCanonical(resolve(cwd, ".heio", "sessions"));
	for (const candidate of withExistingCanonical(absolute)) {
		if (stubPaths.includes(candidate)) return true;
		for (const root of sessionRoots) {
			if (isSessionTodoUnder(root, candidate)) return true;
		}
	}
	return false;
}

export function isProtectedTodoPath(cwd: string, rawPath: string): boolean {
	if (!rawPath) return false;
	return matchesProtectedShape(cwd, resolveTodoToolPath(cwd, rawPath));
}
