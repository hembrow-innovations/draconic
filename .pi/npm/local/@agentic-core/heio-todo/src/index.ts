import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { StringEnum } from "@earendil-works/pi-ai";
import {
	type ExtensionAPI,
	withFileMutationQueue,
} from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import {
	isProtectedTodoPath,
	listSessionChecklists,
	parseSessionId,
	type SessionId,
	sessionTodoPath,
	writeSessionChecklist,
} from "./store.ts";

const SIBLING_LIST_CAP = 5;
const BLOCK_REASON = "Use heio_todo. That path is a session checklist.";

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

function bashMutatesProtected(cwd: string, command: string): boolean {
	const redirect = /(?:>>|>)\s*([^\s;|&]+)/g;
	for (const match of command.matchAll(redirect)) {
		const target = stripQuotes(match[1] ?? "");
		if (target && isProtectedTodoPath(cwd, target)) return true;
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
		if (isProtectedTodoPath(cwd, token)) return true;
	}
	return false;
}

function currentSessionId(raw: string): SessionId | undefined {
	try {
		return parseSessionId(raw);
	} catch {
		return undefined;
	}
}

function formatList(input: {
	cwd: string;
	sessionId: SessionId | undefined;
}): string {
	const items = listSessionChecklists(input.cwd);
	const current = input.sessionId
		? items.find((item) => item.sessionId === input.sessionId)
		: undefined;
	const siblings = items.filter((item) => item.sessionId !== input.sessionId);
	const lines: string[] = [];
	if (current) {
		const body = readFileSync(current.path, "utf8").trimEnd();
		lines.push("This session:");
		lines.push(body);
	} else {
		lines.push("No checklist for this session.");
	}
	if (siblings.length > 0) {
		lines.push("");
		lines.push("Other sessions:");
		const shown = siblings.slice(0, SIBLING_LIST_CAP);
		for (const sibling of shown) {
			lines.push(`- **${sibling.sessionId}**: ${sibling.title}`);
		}
		const extra = siblings.length - shown.length;
		if (extra > 0) {
			lines.push(`- … ${extra} more`);
		}
	}
	return lines.join("\n");
}

export default function (pi: ExtensionAPI) {
	pi.on("tool_call", (event, ctx) => {
		if (event.toolName === "bash") {
			const command = toolCommand(event.input);
			if (!command) return;
			if (!bashMutatesProtected(ctx.cwd, command)) return;
			return { block: true, reason: BLOCK_REASON };
		}
		if (event.toolName !== "write" && event.toolName !== "edit") return;
		const path = toolPath(event.input);
		if (!path) return;
		if (!isProtectedTodoPath(ctx.cwd, path)) return;
		return {
			block: true,
			reason: BLOCK_REASON,
		};
	});

	pi.registerTool({
		name: "heio_todo",
		label: "Heio todo",
		description:
			"Write or list this session's heio checklist. Keep skipped items as `- [ ] skip: reason`.",
		promptSnippet: "Write or list this session's heio checklist",
		promptGuidelines: [
			"Use heio_todo to write the session checklist. Do not write or edit `.heio/TODO.md` or `.heio/sessions/*/TODO.md` with write or edit.",
			"Call heio_todo with action list to see other sessions' checklists. Shared work units live under `.heio/inbox` and `.heio/planning`.",
		],
		parameters: Type.Object({
			action: StringEnum(["write", "list"] as const),
			markdown: Type.Optional(
				Type.String({ description: "Full checklist markdown (for write)." }),
			),
		}),
		async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
			switch (params.action) {
				case "write": {
					const markdown = params.markdown;
					if (!markdown) {
						const text = "markdown is required for action write";
						return {
							content: [{ type: "text" as const, text }],
							details: { error: text },
						};
					}
					let sessionId: SessionId;
					try {
						sessionId = parseSessionId(ctx.sessionManager.getSessionId());
					} catch (error) {
						const message =
							error instanceof Error ? error.message : "invalid session id";
						return {
							content: [{ type: "text" as const, text: message }],
							details: { error: message },
						};
					}
					const sessionPath = resolve(sessionTodoPath(ctx.cwd, sessionId));
					return withFileMutationQueue(sessionPath, async () => {
						const written = writeSessionChecklist({
							cwd: ctx.cwd,
							sessionId,
							markdown,
						});
						return {
							content: [
								{
									type: "text" as const,
									text: `Wrote ${written.sessionPath}`,
								},
							],
							details: written,
						};
					});
				}
				case "list": {
					const sessionId = currentSessionId(ctx.sessionManager.getSessionId());
					const text = formatList({ cwd: ctx.cwd, sessionId });
					const items = listSessionChecklists(ctx.cwd);
					return {
						content: [{ type: "text" as const, text }],
						details: { items, currentSessionId: sessionId },
					};
				}
				default: {
					const _exhaustive: never = params.action;
					const text = `Unknown action: ${String(_exhaustive)}`;
					return {
						content: [{ type: "text" as const, text }],
						details: { error: text },
					};
				}
			}
		},
	});
}
