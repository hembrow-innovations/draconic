#!/usr/bin/env node
// Run OpenCode N times, streaming JSON live. Default sitting is /afk-roadmap.
// Usage: node .loop/opencode-loop.mjs <loops> [prompt...]
//        node .loop/opencode-loop.mjs 200
//        node .loop/opencode-loop.mjs 5 "fix the failing tests"
// Extra opencode flags: put them after `--`, e.g. ... "prompt" -- -m xai/grok-4.6
// SLEEP=<seconds> between loops. STALL_SEC=900 idle stdout/stderr → kill (0 disables).
// STALL_ACTION=continue|abort (default continue). FAIL_ACTION=continue|abort (default continue).
// LOG=0 skips .loop/logs/loop-<stamp>.log (default on).

import { spawn } from "node:child_process";
import { createWriteStream, existsSync, mkdirSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { readRoadmapStatus } from "./roadmap-status.mjs";

const DEFAULT_COMMAND = "afk-roadmap";
const DEFAULT_ARGS = ["continue"];

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
const useDefaultAudit = promptParts.length === 0;

const sleepMs = (Number.parseFloat(process.env.SLEEP) || 0) * 1000;
const stallSec = stallSeconds();
const stallMs = stallSec * 1000;
const stallAction = (process.env.STALL_ACTION || "continue").toLowerCase();
const failAction = (process.env.FAIL_ACTION || "continue").toLowerCase();
const logStream = openLog();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function stallSeconds() {
	const raw = process.env.STALL_SEC;
	if (raw === undefined || raw === "") return 900;
	const n = Number.parseFloat(raw);
	return Number.isFinite(n) && n >= 0 ? n : 900;
}

function stamp() {
	const d = new Date();
	const p = (n) => String(n).padStart(2, "0");
	return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`;
}

function openLog() {
	if (process.env.LOG === "0") return null;
	const dir = join(process.cwd(), ".loop", "logs");
	mkdirSync(dir, { recursive: true });
	const path = join(dir, `loop-${stamp()}.log`);
	const stream = createWriteStream(path, { flags: "a" });
	console.log(`log: ${path} (LOG=0 to disable)`);
	return stream;
}

function emit(line, to = process.stdout) {
	to.write(`${line}\n`);
	logStream?.write(`${line}\n`);
}

function ledgerIds() {
	const ids = new Set();
	const dirs = [
		join(process.cwd(), ".heio/planning/rounds"),
		join(process.cwd(), ".heio/archive/planning/rounds"),
	];
	for (const dir of dirs) {
		if (!existsSync(dir)) continue;
		for (const name of readdirSync(dir)) {
			if (!name.includes("roadmap-audit")) continue;
			const text = readFileSync(join(dir, name), "utf8");
			for (const m of text.matchAll(/^- \*\*([A-Za-z][A-Za-z0-9.]*)\*\*:/gm)) {
				ids.add(m[1]);
			}
		}
	}
	return ids;
}

function campaign() {
	const { items } = readRoadmapStatus();
	const done = ledgerIds();
	const remaining = items.filter((item) => !done.has(item.id));
	return {
		total: items.length,
		audited: [...done].filter((id) => items.some((item) => item.id === id))
			.length,
		remaining: remaining.length,
		next: remaining[0]?.id ?? null,
	};
}

function opencodeArgs() {
	const flags = ["run", "--auto", "--format", "json"];
	if (useDefaultAudit) {
		flags.push("--command", DEFAULT_COMMAND, ...DEFAULT_ARGS);
	} else {
		flags.push(...promptParts);
	}
	flags.push(...extraFlags);
	return flags;
}

function run(i) {
	const args = opencodeArgs();
	return new Promise((resolve) => {
		emit(`\n===== loop ${i}/${loops} =====`);
		const child = spawn("opencode", args, {
			stdio: ["ignore", "pipe", "pipe"],
			detached: true,
		});
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
		const killGroup = (sig) => {
			try {
				if (child.pid) process.kill(-child.pid, sig);
			} catch {
				try {
					child.kill(sig);
				} catch {
					/* already dead */
				}
			}
		};

		createInterface({ input: child.stdout }).on("line", (line) => {
			touch();
			if (!line.trim()) return;
			try {
				emit(JSON.stringify(JSON.parse(line), null, 2));
			} catch {
				emit(line);
			}
		});
		createInterface({ input: child.stderr }).on("line", (line) => {
			touch();
			emit(line, process.stderr);
		});

		const watchdog =
			stallMs > 0
				? setInterval(() => {
						const idle = Date.now() - lastActivity;
						if (idle < stallMs) return;
						stalled = true;
						emit(
							`[stall] loop ${i}: no output for ${Math.round(idle / 1000)}s (limit ${stallSec}s) — killing pid ${child.pid}`,
							process.stderr,
						);
						killGroup("SIGTERM");
						setTimeout(() => killGroup("SIGKILL"), 5000).unref?.();
					}, Math.min(5000, Math.max(1000, stallMs / 4)))
				: null;

		child.on("close", (code) => {
			finish(code ?? 0, stalled ? "stall" : code === 0 ? "ok" : "error");
		});
		child.on("error", (err) => {
			emit(`[error] loop ${i}: ${err.message}`, process.stderr);
			finish(1, "error");
		});
	});
}

const argsPreview = opencodeArgs().join(" ");
emit(`sleep between loops: ${sleepMs / 1000}s (SLEEP=<seconds>)`);
emit(
	`stall watchdog: ${stallSec > 0 ? `${stallSec}s idle → kill (${stallAction})` : "disabled"} (STALL_SEC / STALL_ACTION)`,
);
emit(`prompt: ${useDefaultAudit ? `/afk-roadmap continue` : promptParts.join(" ")}`);
emit(`opencode ${argsPreview}`);

let stalls = 0;
let errors = 0;
for (let i = 1; i <= loops; i++) {
	if (useDefaultAudit) {
		const { total, audited, remaining, next } = campaign();
		emit(`campaign: audited ${audited}/${total} remaining=${remaining} next=${next ?? "none"}`);
		if (remaining === 0) {
			emit("campaign complete — every ROADMAP.md row is in the ledger. stopping.");
			break;
		}
	}
	const { code, reason } = await run(i);
	if (reason === "stall") {
		stalls++;
		emit(`loop ${i} stalled (total stalls: ${stalls})`, process.stderr);
		if (stallAction === "abort") {
			emit("STALL_ACTION=abort — stopping.", process.stderr);
			process.exit(1);
		}
	} else if (code !== 0) {
		errors++;
		emit(`loop ${i} exited with code ${code}`, process.stderr);
		if (failAction === "abort") {
			emit("FAIL_ACTION=abort — stopping.", process.stderr);
			process.exit(code);
		}
	}
	if (sleepMs && i < loops) {
		emit(`sleeping ${sleepMs / 1000}s...`);
		await sleep(sleepMs);
	}
}

emit(`\n===== done: ${loops} loops, ${stalls} stall(s), ${errors} error(s) =====`);
logStream?.end();
if (stalls > 0 || errors > 0) process.exit(1);
