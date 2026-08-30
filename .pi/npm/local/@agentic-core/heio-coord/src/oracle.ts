import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { readStackStatus } from "./status.ts";

export type OracleResult = { ok: boolean; text: string };

function sliceLedger(cwd: string): string | undefined {
	const status = readStackStatus(cwd);
	if (!status.sprintId || !status.sliceId) return undefined;
	return join(
		".heio",
		"planning",
		"sprints",
		status.sprintId,
		"slices",
		status.sliceId,
		"oracles.md",
	);
}

export function runOracle(input: { cwd: string; mode: string }): OracleResult {
	if (input.mode !== "status" && input.mode !== "reverify") {
		return { ok: false, text: "target must be status or reverify" };
	}
	const ledger = sliceLedger(input.cwd);
	if (!ledger) {
		return { ok: false, text: "no slice ledger" };
	}
	const script = join(
		input.cwd,
		".pi",
		"skills",
		"oracle",
		"scripts",
		"oracle-check.mjs",
	);
	if (!existsSync(script)) {
		return { ok: false, text: "missing oracle-check.mjs" };
	}
	const result = spawnSync(
		process.execPath,
		[script, `--${input.mode}`, ledger],
		{
			cwd: input.cwd,
			encoding: "utf8",
			timeout: 120_000,
			env: process.env,
		},
	);
	const text = `${result.stdout ?? ""}${result.stderr ?? ""}`;
	return { ok: result.status === 0, text };
}
