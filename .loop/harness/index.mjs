import { createPaint, wrapAnsi } from "./ansi.mjs"
import { renderMarkdown } from "./markdown.mjs"
import { formatTool } from "./tools.mjs"

export function createHarness(opts = {}) {
	const seen = new Set()
	return {
		format(raw) {
			const paint = createPaint(opts.color ?? envColor())
			const width = opts.width ?? Math.max(40, (process.stdout.columns || 80) - 1)
			const indent = opts.indent ?? "  "
			const ctx = { paint, width, indent }
			const ev = normalize(raw)
			const id = ev.part?.id
			if (id) {
				const key = `${ev.type}:${id}:${ev.part?.state?.status ?? ""}`
				if (seen.has(key)) return ""
				seen.add(key)
			}
			switch (ev.type) {
				case "text": {
					const text = String(ev.part?.text ?? "").trim()
					if (!text) return ""
					return `\n${renderMarkdown(text, ctx)}`
				}
				case "reasoning": {
					const text = String(ev.part?.text ?? "").trim()
					if (!text) return ""
					return `\n${indent}${paint.think("thinking")}\n${renderMarkdown(text, { ...ctx, indent: indent + "  " })}`
				}
				case "tool_use":
					return formatTool(ev.part, ctx)
				case "step_start":
					return paint.dim(`${indent}${"─".repeat(24)}`)
				case "step_finish":
					return ""
				case "error":
					return `${indent}${paint.err(`✗ ${errorMessage(ev.error)}`)}`
				default:
					return formatUnknown(ev, paint, indent, width)
			}
		},
	}
}

function envColor() {
	if (process.env.NO_COLOR) return false
	if (process.env.COLOR === "0") return false
	if (process.env.FORCE_COLOR) return true
	return Boolean(process.stdout.isTTY)
}

function normalize(obj) {
	if (!obj || typeof obj !== "object") return { type: "unknown" }
	if (obj.type === "message.part.updated") {
		const part = obj.properties?.part ?? obj.part
		if (!part) return obj
		if (part.type === "text") return { type: "text", part }
		if (part.type === "reasoning") return { type: "reasoning", part }
		if (part.type === "tool") return { type: "tool_use", part }
		if (part.type === "step-start") return { type: "step_start", part }
		if (part.type === "step-finish") return { type: "step_finish", part }
	}
	if (obj.type === "session.error") {
		return { type: "error", error: obj.properties?.error ?? obj.error }
	}
	return obj
}

function errorMessage(err) {
	if (!err) return "error"
	if (typeof err === "string") return err
	if (err.data?.message) return String(err.data.message)
	if (err.message) return String(err.message)
	if (err.name) return String(err.name)
	try {
		return JSON.stringify(err)
	} catch {
		return "error"
	}
}

function formatUnknown(ev, paint, indent, width) {
	const t = String(ev.type || "event")
	if (
		t.startsWith("session.") ||
		t.startsWith("message.") ||
		t === "server.heartbeat" ||
		t === "server.connected"
	) {
		return ""
	}
	const bits = []
	for (const [k, v] of Object.entries(ev)) {
		if (k === "type" || k === "timestamp" || k === "sessionID") continue
		if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
			bits.push(`${k}=${String(v).slice(0, 48)}`)
		}
		if (bits.length >= 3) break
	}
	return wrapAnsi(paint.dim(`· ${t}${bits.length ? " " + bits.join(" ") : ""}`), width, indent)
}
