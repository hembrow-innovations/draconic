#!/usr/bin/env node
// Run the same OpenCode prompt N times, streaming all output live.
// Usage: node .loop/opencode-loop.mjs <loops> [prompt...]
//        node .loop/opencode-loop.mjs 5
//        node .loop/opencode-loop.mjs 5 "run the draconic-loop skill once"
// Extra opencode flags: put them after `--`, e.g. ... "prompt" -- -m xai/grok-4.5
// Optional sleep between loops (seconds): SLEEP=60 node .loop/opencode-loop.mjs 5
// Stall watchdog (seconds of no stdout): STALL_SEC=600 (default 600). Set 0 to disable.
// On stall: kill the child, log, continue to next loop (STALL_ACTION=continue|abort, default continue).

import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

const DEFAULT_PROMPT =
	"Run the draconic-loop skill exactly once: claim the next ROADMAP.md item, implement it test-first, mark it done only when cargo test --workspace is green, then stop. Do not start a second item.";

// Full Test262 is opt-in now; 15m still covers slow compiles without false stalls.
const DEFAULT_STALL_SEC = 900;

const [, , loopsArg, ...rest] = process.argv;
const loops = Number.parseInt(loopsArg, 10);
if (!Number.isInteger(loops) || loops < 1) {
	console.error(
		"Usage: node .loop/opencode-loop.mjs <loops> [prompt...] [-- <opencode flags>]",
	);
	process.exit(1);
}

const dash = rest.indexOf("--");
const promptParts = dash === -1 ? rest : rest.slice(0, dash);
const extraFlags = dash === -1 ? [] : rest.slice(dash + 1);
const promptArgs = promptParts.length > 0 ? promptParts : [DEFAULT_PROMPT];

const stallSec = (() => {
	const raw = process.env.STALL_SEC;
	if (raw === undefined || raw === "") return DEFAULT_STALL_SEC;
	const n = Number.parseFloat(raw);
	return Number.isFinite(n) && n >= 0 ? n : DEFAULT_STALL_SEC;
})();
const stallMs = stallSec * 1000;
const stallAction = (process.env.STALL_ACTION || "continue").toLowerCase();

const flags = [
	// auto-approve permissions that are not explicitly denied
	"--auto",
	// stream events live as JSON instead of formatted text
	"--format",
	"json",
];

const run = (i) =>
	new Promise((resolve) => {
		process.stdout.write(`\n===== loop ${i}/${loops} =====\n`);
		// pipe stdout so we can pretty-print each JSONL event as it arrives.
		const child = spawn(
			"opencode",
			["run", ...flags, ...promptArgs, ...extraFlags],
			{
				stdio: ["inherit", "pipe", "inherit"],
			},
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
				console.log(line); // not JSON — print raw
			}
		});

		const watchdog =
			stallMs > 0
				? setInterval(() => {
						const idle = Date.now() - lastActivity;
						if (idle < stallMs) return;
						stalled = true;
						console.error(
							`\n[stall] loop ${i}: no stdout for ${Math.round(idle / 1000)}s (limit ${stallSec}s) — killing pid ${child.pid}`,
						);
						try {
							child.kill("SIGTERM");
						} catch {
							/* already dead */
						}
						// hard kill if it ignores SIGTERM
						setTimeout(() => {
							try {
								child.kill("SIGKILL");
							} catch {
								/* already dead */
							}
						}, 5000).unref?.();
					}, Math.min(5000, Math.max(1000, stallMs / 4)))
				: null;

		// resolve on "close" (stdout drained), not "exit" (can fire while lines pending)
		child.on("close", (code) => {
			if (stalled) {
				finish(code ?? 1, "stall");
				return;
			}
			finish(code ?? 0, code === 0 ? "ok" : "error");
		});
		child.on("error", (err) => {
			console.error(`[error] loop ${i}: ${err.message}`);
			finish(1, "error");
		});
	});

const sleepMs = (Number.parseFloat(process.env.SLEEP) || 0) * 1000;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
console.log(
	`sleep between loops: ${sleepMs / 1000}s (set with SLEEP=<seconds>)`,
);
console.log(
	`stall watchdog: ${stallSec > 0 ? `${stallSec}s idle → kill (${stallAction})` : "disabled"} (STALL_SEC / STALL_ACTION)`,
);
console.log(`prompt: ${promptArgs.join(" ")}`);

let stalls = 0;
let errors = 0;
for (let i = 1; i <= loops; i++) {
	const { code, reason } = await run(i);
	if (reason === "stall") {
		stalls++;
		console.error(`loop ${i} stalled (total stalls: ${stalls})`);
		if (stallAction === "abort") {
			console.error("STALL_ACTION=abort — stopping.");
			process.exit(1);
		}
	} else if (code !== 0) {
		errors++;
		console.error(`loop ${i} exited with code ${code}`);
	}
	if (sleepMs && i < loops) {
		console.log(`sleeping ${sleepMs / 1000}s...`);
		await sleep(sleepMs);
	}
}

console.log(
	`\n===== done: ${loops} loops, ${stalls} stall(s), ${errors} error(s) =====`,
);
if (stalls > 0 || errors > 0) process.exit(1);
