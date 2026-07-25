#!/usr/bin/env node
// Run the same OpenCode prompt N times, streaming all output live.
// Usage: node .loop/opencode-loop.mjs <loops> [prompt...]
//        node .loop/opencode-loop.mjs 5
//        node .loop/opencode-loop.mjs 5 "run the draconic-loop skill once"
// Extra opencode flags: put them after `--`, e.g. ... "prompt" -- -m xai/grok-4.5
// Optional sleep between loops (seconds): SLEEP=60 node .loop/opencode-loop.mjs 5

import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

const DEFAULT_PROMPT =
	"Run the draconic-loop skill exactly once: claim the next ROADMAP.md item, implement it test-first, mark it done only when cargo test --workspace is green, then stop. Do not start a second item.";

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
		createInterface({ input: child.stdout }).on("line", (line) => {
			if (!line.trim()) return;
			try {
				console.log(JSON.stringify(JSON.parse(line), null, 2));
			} catch {
				console.log(line); // not JSON — print raw
			}
		});
		// resolve on "close" (stdout drained), not "exit" (can fire while lines pending)
		child.on("close", (code) => resolve(code ?? 0));
	});

const sleepMs = (Number.parseFloat(process.env.SLEEP) || 0) * 1000;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
console.log(
	`sleep between loops: ${sleepMs / 1000}s (set with SLEEP=<seconds>)`,
);
console.log(`prompt: ${promptArgs.join(" ")}`);

for (let i = 1; i <= loops; i++) {
	const code = await run(i);
	if (code !== 0) console.error(`loop ${i} exited with code ${code}`);
	if (sleepMs && i < loops) {
		console.log(`sleeping ${sleepMs / 1000}s...`);
		await sleep(sleepMs);
	}
}
