import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { delimiter, join } from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";

const MISSING = "onic is not installed";

function resolveOnicBinary(env: NodeJS.ProcessEnv): string | undefined {
	const path = env.PATH ?? "";
	for (const dir of path.split(delimiter)) {
		if (!dir) continue;
		const candidate = join(dir, "onic");
		if (existsSync(candidate)) return candidate;
	}
	return undefined;
}

export default function (pi: ExtensionAPI) {
	pi.registerTool({
		name: "heio_onic",
		label: "Heio onic",
		description:
			"Query a code neighborhood via onic. Missing binary fails closed with a reason.",
		promptSnippet: "Query a code neighborhood with heio_onic",
		parameters: Type.Object({
			action: Type.Union([
				Type.Literal("schema"),
				Type.Literal("compact"),
				Type.Literal("search"),
			]),
			query: Type.Optional(Type.String()),
		}),
		async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
			const binary = resolveOnicBinary(process.env);
			if (!binary) {
				return {
					content: [{ type: "text" as const, text: MISSING }],
					details: { error: MISSING },
				};
			}
			const args = params.query ? [params.action, params.query] : [params.action];
			const result = spawnSync(binary, args, {
				cwd: ctx.cwd,
				encoding: "utf8",
			});
			const text = result.stdout;
			return {
				content: [{ type: "text" as const, text }],
				details: { error: result.status === 0 ? "" : MISSING },
			};
		},
	});
}
