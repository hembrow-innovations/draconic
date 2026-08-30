import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
	CONFIG_DIR_NAME,
	type ExtensionAPI,
	type ExtensionContext,
	getAgentDir,
} from "@earendil-works/pi-coding-agent";
import {
	clipToVisibleWidth,
	formatCwdFromRoot,
	formatFooterLine,
} from "./format.ts";

type CompactionRead =
	| { kind: "missing" }
	| { kind: "invalid" }
	| { kind: "value"; enabled: boolean };

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null;
}

function compactionEnabledFromFile(path: string): CompactionRead {
	if (!existsSync(path)) return { kind: "missing" };
	try {
		const raw: unknown = JSON.parse(readFileSync(path, "utf8"));
		if (!raw || typeof raw !== "object") return { kind: "invalid" };
		if (!("compaction" in raw)) return { kind: "missing" };
		const compaction = raw.compaction;
		if (!compaction || typeof compaction !== "object") return { kind: "invalid" };
		if (!("enabled" in compaction)) return { kind: "missing" };
		return typeof compaction.enabled === "boolean"
			? { kind: "value", enabled: compaction.enabled }
			: { kind: "invalid" };
	} catch {
		return { kind: "invalid" };
	}
}

function autoCompactEnabled(cwd: string): boolean {
	const project = compactionEnabledFromFile(
		join(cwd, CONFIG_DIR_NAME, "settings.json"),
	);
	if (project.kind === "value") return project.enabled;
	if (project.kind === "invalid") return false;
	const global = compactionEnabledFromFile(join(getAgentDir(), "settings.json"));
	if (global.kind === "value") return global.enabled;
	if (global.kind === "invalid") return false;
	return true;
}

function usageTotal(usage: unknown): number {
	if (!isRecord(usage) || !isRecord(usage.cost)) return 0;
	return typeof usage.cost.total === "number" ? usage.cost.total : 0;
}

function sessionCost(ctx: ExtensionContext): number {
	let cost = 0;
	for (const entry of ctx.sessionManager.getEntries()) {
		if (!isRecord(entry) || typeof entry.type !== "string") continue;
		if (entry.type === "message" && isRecord(entry.message)) {
			if (entry.message.role === "assistant") {
				cost += usageTotal(entry.message.usage);
			} else if (entry.message.role === "toolResult") {
				cost += usageTotal(entry.message.usage);
			}
			continue;
		}
		if (entry.type === "branch_summary" || entry.type === "compaction") {
			cost += usageTotal(entry.usage);
		}
	}
	return cost;
}

export default function (pi: ExtensionAPI) {
	pi.on("session_start", (_event, ctx) => {
		if (ctx.mode !== "tui") return;
		const autoCompact = autoCompactEnabled(ctx.cwd);
		ctx.ui.setFooter((_tui, theme, footerData) => ({
			invalidate() {},
			render(width: number): string[] {
				const usage = ctx.getContextUsage();
				const line = formatFooterLine({
					cwd: formatCwdFromRoot(ctx.cwd),
					teamStatus: footerData.getExtensionStatuses().get("team"),
					tokens: usage?.tokens ?? null,
					contextWindow: usage?.contextWindow ?? ctx.model?.contextWindow ?? 0,
					cost: sessionCost(ctx),
					autoCompact,
					model: ctx.model?.id ?? "no-model",
					effort: ctx.thinkingLevel,
				});
				const clipped = clipToVisibleWidth(line, width);
				return [theme.fg("dim", clipped)];
			},
		}));
	});
}
