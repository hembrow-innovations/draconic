import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { test } from "node:test";
import {
	attachWatchdog,
	createWatchdog,
	isActivityLine,
	parseIdleMs,
} from "./watchdog.mjs";

test("default idle is 30 minutes", () => {
	assert.equal(parseIdleMs({}), 1_800_000);
});

test("IDLE=0 disables the watchdog", () => {
	assert.equal(parseIdleMs({ IDLE: "0" }), 0);
});

test("IDLE seconds become milliseconds", () => {
	assert.equal(parseIdleMs({ IDLE: "90" }), 90_000);
});

test("heartbeat JSON is not activity", () => {
	assert.equal(isActivityLine('{"type":"server.heartbeat"}'), false);
	assert.equal(isActivityLine('{"type":"server.connected"}'), false);
});

test("tool JSON and plain text are activity", () => {
	assert.equal(isActivityLine('{"type":"tool_use"}'), true);
	assert.equal(isActivityLine("not json"), true);
	assert.equal(isActivityLine("  "), false);
});

test("idle fires once after silence", () => {
	let now = 0;
	const idles = [];
	const beats = [];
	const w = createWatchdog({
		idleMs: 1000,
		beatMs: 400,
		now: () => now,
		onBeat: (ms) => beats.push(ms),
		onIdle: (ms) => idles.push(ms),
	});
	now = 400;
	assert.equal(w.tick(), false);
	now = 800;
	assert.equal(w.tick(), false);
	now = 1000;
	assert.equal(w.tick(), true);
	now = 2000;
	assert.equal(w.tick(), false);
	assert.deepEqual(idles, [1000]);
	assert.deepEqual(beats, [400, 800]);
});

test("activity resets silence", () => {
	let now = 0;
	const idles = [];
	const beats = [];
	const w = createWatchdog({
		idleMs: 1000,
		beatMs: 400,
		now: () => now,
		onBeat: (ms) => beats.push(ms),
		onIdle: (ms) => idles.push(ms),
	});
	now = 900;
	w.pet('{"type":"text"}');
	now = 1200;
	assert.equal(w.tick(), false);
	now = 1900;
	assert.equal(w.tick(), true);
	assert.deepEqual(idles, [1000]);
	assert.deepEqual(beats, []);
});

test("heartbeat does not reset silence", () => {
	let now = 0;
	const w = createWatchdog({
		idleMs: 1000,
		beatMs: 0,
		now: () => now,
		onIdle: () => {},
	});
	now = 500;
	w.pet('{"type":"server.heartbeat"}');
	now = 1000;
	assert.equal(w.tick(), true);
});

test("idleMs 0 never fires", () => {
	let now = 0;
	const w = createWatchdog({
		idleMs: 0,
		beatMs: 100,
		now: () => now,
		onIdle: () => {
			throw new Error("fired");
		},
	});
	now = 10_000;
	assert.equal(w.tick(), false);
});

test("kills a silent child and continues", async () => {
	const child = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], {
		stdio: ["ignore", "pipe", "ignore"],
	});
	const killed = await new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			child.kill("SIGKILL");
			reject(new Error("watchdog did not kill"));
		}, 2000);
		attachWatchdog(child, {
			idleMs: 80,
			beatMs: 0,
			tickMs: 20,
			killAfterMs: 50,
			onIdle: () => {},
		});
		child.on("close", () => {
			clearTimeout(timeout);
			resolve(true);
		});
	});
	assert.equal(killed, true);
});
