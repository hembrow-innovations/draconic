import { createInterface } from "node:readline";

export const DEFAULT_IDLE_MS = 1_800_000;

export function parseIdleMs(env = process.env) {
	if (env.IDLE == null || env.IDLE === "") return DEFAULT_IDLE_MS;
	const n = Number.parseFloat(env.IDLE);
	if (!Number.isFinite(n) || n < 0) return DEFAULT_IDLE_MS;
	return n * 1000;
}

export function isActivityLine(line) {
	const t = String(line ?? "").trim();
	if (!t) return false;
	try {
		const type = JSON.parse(t)?.type;
		return type !== "server.heartbeat" && type !== "server.connected";
	} catch {
		return true;
	}
}

export function createWatchdog({
	idleMs,
	beatMs = 60_000,
	now = Date.now,
	onBeat,
	onIdle,
}) {
	let last = now();
	let lastBeat = last;
	let fired = false;
	return {
		pet(line) {
			if (fired) return;
			if (line != null && !isActivityLine(line)) return;
			last = now();
			lastBeat = last;
		},
		tick() {
			if (fired || idleMs <= 0) return false;
			const t = now();
			const silent = t - last;
			if (silent >= idleMs) {
				fired = true;
				onIdle?.(silent);
				return true;
			}
			if (beatMs > 0 && t - lastBeat >= beatMs) {
				lastBeat = t;
				onBeat?.(silent);
			}
			return false;
		},
	};
}

export function attachWatchdog(child, opts = {}) {
	const idleMs = opts.idleMs ?? 0;
	const watchdog = createWatchdog({
		idleMs,
		beatMs: opts.beatMs ?? 60_000,
		now: opts.now,
		onBeat: opts.onBeat,
		onIdle: (silent) => {
			opts.onIdle?.(silent);
			killChild(child, opts.killAfterMs ?? 5000);
		},
	});
	const rl = createInterface({ input: child.stdout });
	rl.on("line", (line) => {
		watchdog.pet(line);
		opts.onLine?.(line);
	});
	const timer =
		idleMs > 0
			? setInterval(() => watchdog.tick(), opts.tickMs ?? 1000)
			: null;
	const stop = () => {
		if (timer) clearInterval(timer);
		rl.close();
	};
	child.on("close", stop);
	return { stop, watchdog };
}

function killChild(child, graceMs) {
	try {
		child.kill("SIGTERM");
	} catch {}
	const killer = setTimeout(() => {
		try {
			child.kill("SIGKILL");
		} catch {}
	}, graceMs);
	killer.unref?.();
	child.once("close", () => clearTimeout(killer));
}
