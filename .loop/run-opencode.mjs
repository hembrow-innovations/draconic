#!/usr/bin/env node
// Shared helper: spawn one `opencode run` with JSON stream + stall watchdog.
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

export const DEFAULT_LOOP_PROMPT =
	"Run the draconic-loop skill exactly once: claim the next ROADMAP.md item, implement it test-first, mark it done only when cargo test --workspace is green, then commit and stop. Do not start a second item. If there is no todo item left, report empty board and stop immediately.";

export function stallConfig() {
	const raw = process.env.STALL_SEC;
	const DEFAULT_STALL_SEC = 900;
	let stallSec = DEFAULT_STALL_SEC;
	if (raw !== undefined && raw !== "") {
		const n = Number.parseFloat(raw);
		if (Number.isFinite(n) && n >= 0) stallSec = n;
	}
	return {
		stallSec,
		stallMs: stallSec * 1000,
		stallAction: (process.env.STALL_ACTION || "continue").toLowerCase(),
	};
}

/**
 * @param {{ label: string, promptArgs: string[], extraFlags?: string[], quiet?: boolean }} opts
 * @returns {Promise<{ code: number, reason: string }>}
 */
export function runOpencodeOnce(opts) {
	const { label, promptArgs, extraFlags = [], quiet = false } = opts;
	const { stallSec, stallMs } = stallConfig();

	return new Promise((resolve) => {
		if (!quiet) process.stdout.write(`\n===== ${label} =====\n`);
		const child = spawn(
			"opencode",
			["run", "--auto", "--format", "json", ...promptArgs, ...extraFlags],
			{ stdio: ["inherit", "pipe", "inherit"] },
		);

		let lastActivity = Date.now();
		let stalled = false;
		let settled = false;
		const finish = (code, reason) => {
			if (settled) return;
			settled = true;
			if (watchdog) clearInterval(watchdog);
			resolve({ code: code ?? 0, reason });
		};
		const touch = () => {
			lastActivity = Date.now();
		};

		createInterface({ input: child.stdout }).on("line", (line) => {
			touch();
			if (!line.trim()) return;
			try {
				console.log(JSON.stringify(JSON.parse(line), null, 2));
			} catch {
				console.log(line);
			}
		});

		const watchdog =
			stallMs > 0
				? setInterval(() => {
						const idle = Date.now() - lastActivity;
						if (idle < stallMs) return;
						stalled = true;
						console.error(
							`\n[stall] ${label}: no stdout for ${Math.round(idle / 1000)}s (limit ${stallSec}s) — killing pid ${child.pid}`,
						);
						try {
							child.kill("SIGTERM");
						} catch {
							/* already dead */
						}
						setTimeout(() => {
							try {
								child.kill("SIGKILL");
							} catch {
								/* already dead */
							}
						}, 5000).unref?.();
					}, Math.min(5000, Math.max(1000, stallMs / 4)))
				: null;

		child.on("close", (code) => {
			if (stalled) {
				finish(code ?? 1, "stall");
				return;
			}
			finish(code ?? 0, code === 0 ? "ok" : "error");
		});
		child.on("error", (err) => {
			console.error(`[error] ${label}: ${err.message}`);
			finish(1, "error");
		});
	});
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
