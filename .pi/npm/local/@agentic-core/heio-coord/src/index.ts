import { StringEnum } from "@earendil-works/pi-ai";
import type {
	ExtensionAPI,
	ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import { withFileMutationQueue } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { advanceSlice, isBuilderShaped } from "./advance.ts";
import { claimTask, releaseTask } from "./claim.ts";
import { blockIllegalWrite } from "./enforce.ts";
import { runOracle } from "./oracle.ts";
import { createTicket } from "./ticket.ts";
import { formatStackStatus, readStackStatus } from "./status.ts";
import { inertReason, isForeignTracker, namedTracker } from "./tracker.ts";
import { recordVerdict } from "./verdict.ts";

function statusText(cwd: string): string {
	const named = namedTracker(cwd);
	if (isForeignTracker(named)) {
		return `inert: ${inertReason(named)}`;
	}
	return formatStackStatus(readStackStatus(cwd));
}

function notify(ctx: Pick<ExtensionContext, "ui">, text: string): void {
	try {
		ctx.ui.notify(text, "info");
	} catch {
		// print mode
	}
}

function builderFrom(piApi: ExtensionAPI, ctx: ExtensionContext): boolean {
	const prompt =
		typeof ctx.getSystemPrompt === "function" ? ctx.getSystemPrompt() : "";
	const sessionName =
		typeof ctx.sessionManager?.getSessionName === "function"
			? ctx.sessionManager.getSessionName()
			: "";
	const agent =
		typeof piApi.getFlag === "function" ? piApi.getFlag("agent") : undefined;
	return isBuilderShaped({
		prompt,
		sessionName,
		agent: typeof agent === "string" ? agent : undefined,
	});
}

export default function (pi: ExtensionAPI) {
	let announcedInert = false;

	pi.on("tool_call", (event, ctx) => {
		return blockIllegalWrite(event, ctx);
	});

	pi.on("session_start", (_event, ctx) => {
		const named = namedTracker(ctx.cwd);
		if (!isForeignTracker(named) || announcedInert) return;
		announcedInert = true;
		notify(ctx, `inert: ${inertReason(named)}`);
	});

	pi.registerTool({
		name: "heio_stack",
		label: "Heio stack",
		description:
			"Heio-stack lens and rails. action status reports active sprint, slice, freeze, and open tickets. claim and release bind a slice task or ticket to this session. oracle runs oracle-check.mjs --status or --reverify on the slice ledger. verdict records TASK, TICKET, ESCALATE, or VERIFY plus one-line evidence. advance is the only status flip (frozen → active → met / abandoned) and refuses a builder-shaped session. Same as /heio for status.",
		promptSnippet: "Read heio-stack status with heio_stack action status",
		promptGuidelines: [
			"Use heio_stack action status instead of opening the planning tree.",
			"Use heio_stack claim and release for slice tasks and tickets.",
			"Use heio_stack action oracle with target status or reverify instead of running oracle-check.mjs.",
			"Use heio_stack action verdict with target TASK, TICKET, ESCALATE, or VERIFY and one-line evidence.",
			"Use heio_stack action advance with target active, met, or abandoned. Builder-shaped sessions cannot mark a slice met.",
			"This tool does not write intent, roadmap, or sprint shape.md.",
			"If AGENTS.md or WORKSPACE.md names another tracker, the coordinator stays inert and says so once.",
		],
		parameters: Type.Object({
			action: StringEnum([
				"status",
				"claim",
				"release",
				"ticket",
				"oracle",
				"verdict",
				"advance",
			] as const),
			target: Type.Optional(
				Type.String({
					description:
						"Slice task id or ticket id for claim/release. status or reverify for oracle. TASK, TICKET, ESCALATE, or VERIFY for verdict. active, met, or abandoned for advance.",
				}),
			),
			evidence: Type.Optional(
				Type.String({
					description: "One-line evidence for action verdict.",
				}),
			),
		}),
		async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
			switch (params.action) {
				case "status": {
					const text = statusText(ctx.cwd);
					return {
						content: [{ type: "text" as const, text }],
						details: { action: "status" },
					};
				}
				case "claim": {
					const named = namedTracker(ctx.cwd);
					if (isForeignTracker(named)) {
						const text = `inert: ${inertReason(named)}`;
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "claim", error: text },
						};
					}
					const target = params.target;
					if (!target) {
						const text = "target is required for action claim";
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "claim", error: text },
						};
					}
					const sessionId = ctx.sessionManager.getSessionId();
					const tasksPath = `${ctx.cwd}/.heio/planning`;
					return withFileMutationQueue(tasksPath, async () => {
						const result = claimTask({
							cwd: ctx.cwd,
							sessionId,
							target,
						});
						return {
							content: [{ type: "text" as const, text: result.text }],
							details: { action: "claim", ok: result.ok },
						};
					});
				}
				case "release": {
					const named = namedTracker(ctx.cwd);
					if (isForeignTracker(named)) {
						const text = `inert: ${inertReason(named)}`;
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "release", error: text },
						};
					}
					const target = params.target;
					if (!target) {
						const text = "target is required for action release";
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "release", error: text },
						};
					}
					const sessionId = ctx.sessionManager.getSessionId();
					const tasksPath = `${ctx.cwd}/.heio/planning`;
					return withFileMutationQueue(tasksPath, async () => {
						const result = releaseTask({
							cwd: ctx.cwd,
							sessionId,
							target,
						});
						return {
							content: [{ type: "text" as const, text: result.text }],
							details: { action: "release", ok: result.ok },
						};
					});
				}
				case "oracle": {
					const named = namedTracker(ctx.cwd);
					if (isForeignTracker(named)) {
						const text = `inert: ${inertReason(named)}`;
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "oracle", error: text },
						};
					}
					const target = params.target;
					if (!target) {
						const text = "target is required for action oracle";
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "oracle", error: text },
						};
					}
					const result = runOracle({ cwd: ctx.cwd, mode: target });
					return {
						content: [{ type: "text" as const, text: result.text }],
						details: { action: "oracle", ok: result.ok },
					};
				}
				case "verdict": {
					const named = namedTracker(ctx.cwd);
					if (isForeignTracker(named)) {
						const text = `inert: ${inertReason(named)}`;
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "verdict", error: text },
						};
					}
					const target = params.target;
					if (!target) {
						const text = "target is required for action verdict";
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "verdict", error: text },
						};
					}
					const evidence = params.evidence;
					if (!evidence) {
						const text = "evidence is required for action verdict";
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "verdict", error: text },
						};
					}
					const result = recordVerdict({ kind: target, evidence });
					return {
						content: [{ type: "text" as const, text: result.text }],
						details: { action: "verdict", ok: result.ok },
					};
				}
				case "advance": {
					const named = namedTracker(ctx.cwd);
					if (isForeignTracker(named)) {
						const text = `inert: ${inertReason(named)}`;
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "advance", error: text },
						};
					}
					const target = params.target;
					if (!target) {
						const text = "target is required for action advance";
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "advance", error: text },
						};
					}
					const planningPath = `${ctx.cwd}/.heio/planning`;
					const builder = builderFrom(pi, ctx);
					return withFileMutationQueue(planningPath, async () => {
						const result = advanceSlice({
							cwd: ctx.cwd,
							target,
							builder,
						});
						return {
							content: [{ type: "text" as const, text: result.text }],
							details: { action: "advance", ok: result.ok },
						};
					});
				}
				case "ticket": {
					const named = namedTracker(ctx.cwd);
					if (isForeignTracker(named)) {
						const text = `inert: ${inertReason(named)}`;
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "ticket", error: text },
						};
					}
					const target = params.target;
					if (!target) {
						const text = "target is required for action ticket";
						return {
							content: [{ type: "text" as const, text }],
							details: { action: "ticket", error: text },
						};
					}
					const ticketsPath = `${ctx.cwd}/.heio/tickets`;
					return withFileMutationQueue(ticketsPath, async () => {
						const result = createTicket({ cwd: ctx.cwd, slug: target });
						return {
							content: [{ type: "text" as const, text: result.text }],
							details: { action: "ticket", ok: result.ok },
						};
					});
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

	pi.registerCommand("heio", {
		description: "Show heio-stack status (same as heio_stack status /heio)",
		async handler(_args, ctx) {
			notify(ctx, statusText(ctx.cwd));
		},
	});
}
