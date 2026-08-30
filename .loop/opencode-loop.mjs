#!/usr/bin/env node
// Run the same pi prompt N times, streaming all output live.
// Usage: node .loop/opencode-loop.mjs <loops> [prompt...]
//        node .loop/opencode-loop.mjs 5
//        node .loop/opencode-loop.mjs 5 "run the draconic-loop skill once"
// Extra pi flags: put them after `--`, e.g. ... "prompt" -- --provider xai --model grok-4.6
// Optional sleep between loops (seconds): SLEEP=60 node .loop/opencode-loop.mjs 5
// Stall watchdog (seconds of no stdout): STALL_SEC=900 (default 900). Set 0 to disable.
// On stall: kill the child, log, continue to next loop (STALL_ACTION=continue|abort, default continue).

import {
	DEFAULT_LOOP_PROMPT,
	runPiOnce,
	sleep,
	stallConfig,
} from "./run-opencode.mjs";

const [, , loopsArg, ...rest] = process.argv;
const loops = Number.parseInt(loopsArg, 10);
if (!Number.isInteger(loops) || loops < 1) {
	console.error(
		"Usage: node .loop/opencode-loop.mjs <loops> [prompt...] [-- <pi flags>]",
	);
	process.exit(1);
}

const dash = rest.indexOf("--");
const promptParts = dash === -1 ? rest : rest.slice(0, dash);
const extraFlags = dash === -1 ? [] : rest.slice(dash + 1);
const promptArgs = promptParts.length > 0 ? promptParts : [DEFAULT_LOOP_PROMPT];

const { stallSec, stallAction } = stallConfig();
const sleepMs = (Number.parseFloat(process.env.SLEEP) || 0) * 1000;

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
	const { code, reason } = await runPiOnce({
		label: `loop ${i}/${loops}`,
		promptArgs,
		extraFlags,
		name: `draconic-loop-${i}-of-${loops}`,
	});
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
