import { homedir } from "node:os"
import { relative, resolve, sep } from "node:path"
import { clipLines, wrapAnsi } from "./ansi.mjs"
import { renderDiff, renderMarkdown } from "./markdown.mjs"

const ICONS = {
	bash: "$",
	shell: "$",
	read: "→",
	write: "←",
	edit: "←",
	apply_patch: "%",
	glob: "✱",
	grep: "✱",
	list: "→",
	webfetch: "%",
	websearch: "◈",
	skill: "→",
	todowrite: "#",
	todo: "#",
	task: "✓",
	lsp: "→",
	question: "?",
	batch: "#",
}

const QUIET = new Set(["read", "glob", "grep", "list", "lsp", "skill", "webfetch", "websearch"])

export function formatTool(part, { paint, width, indent = "  " }) {
	const state = obj(part?.state)
	const input = obj(state.input)
	const meta = obj(state.metadata)
	const status = String(state.status || "")
	const name = String(part?.tool || "tool")
	const icon = status === "error" ? "✗" : status === "running" ? "•" : (ICONS[name] ?? "⚙")
	const title = titleFor(name, input, meta, state)
	const color = status === "error" ? paint.err : status === "running" ? paint.yellow : paint.cyan
	const time = fmtTime(state.time)
	let out = `${indent}${paint.dim(icon)} ${color(title)}${time ? paint.dim(` · ${time}`) : ""}`
	if (status === "error" && state.error) {
		out += `\n${wrapAnsi(paint.err(String(state.error)), width, indent + "  ")}`
	}
	const body = toolBody(name, state, input, meta, paint, width, indent)
	if (body) out += `\n${body}`
	return out
}

function toolBody(name, state, input, meta, paint, width, indent) {
	const hang = indent + "  "
	if (name === "todowrite" || name === "todo") {
		const todos = Array.isArray(input.todos) ? input.todos : []
		if (!todos.length) return ""
		return todos
			.map((item) => {
				const mark = item?.status === "completed" ? paint.ok("☑") : item?.status === "in_progress" ? paint.yellow("•") : paint.dim("☐")
				return wrapAnsi(`${mark} ${item?.content ?? ""}`, width, hang)
			})
			.join("\n")
	}
	if (name === "edit" && typeof meta.diff === "string" && meta.diff.trim()) {
		return indentLines(renderDiff(clipLines(meta.diff, 60), paint), hang)
	}
	if (name === "apply_patch" && Array.isArray(meta.files)) {
		return meta.files
			.slice(0, 8)
			.map((file) => hang + paint.dim(patchLine(file)))
			.join("\n")
	}
	if (name === "task") {
		const result = taskResult(state.output)
		if (!result) return ""
		return renderMarkdown(result, { paint, width, indent: hang })
	}
	if (name === "write" && typeof input.content === "string" && input.content.trim()) {
		return indentLines(paint.dim(clipLines(input.content, 40)), hang)
	}
	if (QUIET.has(name)) return ""
	const output = typeof state.output === "string" ? state.output.trim() : ""
	if (!output) return ""
	if (looksMarkdown(output)) {
		return renderMarkdown(clipLines(output, 80), { paint, width, indent: hang })
	}
	const shown = clipLines(output, 80)
	if (looksDiff(shown)) return indentLines(renderDiff(shown, paint), hang)
	return indentLines(paint.dim(shown), hang)
}

function titleFor(name, input, meta, state) {
	if (name === "bash" || name === "shell") return input.command || state.title || "bash"
	if (name === "read") return `Read ${shortPath(input.filePath)}`
	if (name === "write") return `Write ${shortPath(input.filePath)}`
	if (name === "edit") return `Edit ${shortPath(input.filePath)}`
	if (name === "glob") {
		const root = input.path ? ` in ${shortPath(input.path)}` : ""
		const n = meta.count
		const extra = typeof n === "number" ? ` · ${n} match${n === 1 ? "" : "es"}` : ""
		return `Glob "${input.pattern ?? ""}"${root}${extra}`
	}
	if (name === "grep") {
		const root = input.path ? ` in ${shortPath(input.path)}` : ""
		const n = meta.matches
		const extra = typeof n === "number" ? ` · ${n} match${n === 1 ? "" : "es"}` : ""
		return `Grep "${input.pattern ?? ""}"${root}${extra}`
	}
	if (name === "list") return input.path ? `List ${shortPath(input.path)}` : "List"
	if (name === "webfetch") return input.url ? `WebFetch ${input.url}` : "WebFetch"
	if (name === "websearch") return input.query ? `Search "${input.query}"` : "Search"
	if (name === "skill") return `Skill "${input.name ?? ""}"`
	if (name === "task") return input.description || `${titlecase(input.subagent_type || "task")} Task`
	if (name === "todowrite" || name === "todo") return "Todos"
	if (name === "lsp") return state.title || `LSP ${input.operation || "request"}`
	if (name === "question") {
		const n = Array.isArray(input.questions) ? input.questions.length : 0
		return `Asked ${n} question${n === 1 ? "" : "s"}`
	}
	if (typeof state.title === "string" && state.title.trim()) return state.title.trim()
	for (const v of Object.values(input)) {
		if (typeof v === "string" && v.trim() && v.length < 80) return `${name} ${v.trim()}`
	}
	return name
}

function shortPath(input) {
	if (!input || typeof input !== "string") return ""
	const cwd = process.cwd()
	const abs = input.startsWith("/") ? input : resolve(cwd, input)
	const rel = relative(cwd, abs)
	if (rel && !rel.startsWith(".." + sep) && rel !== "..") return rel.replaceAll("\\", "/")
	const home = homedir()
	if (home && (abs === home || abs.startsWith(home + sep))) {
		return abs.replace(home, "~").replaceAll("\\", "/")
	}
	return abs.replaceAll("\\", "/")
}

function fmtTime(time) {
	if (!time || typeof time.start !== "number" || typeof time.end !== "number") return ""
	const ms = time.end - time.start
	if (ms < 0 || !Number.isFinite(ms)) return ""
	if (ms < 1000) return `${Math.round(ms)}ms`
	if (ms < 10_000) return `${(ms / 1000).toFixed(1)}s`
	return `${Math.round(ms / 1000)}s`
}

function taskResult(output) {
	if (typeof output !== "string" || !output.trim()) return ""
	const match = output.match(/<task_result>\s*([\s\S]*?)\s*<\/task_result>/)
	if (match) return match[1].trim()
	return output
		.split("\n")
		.filter((line) => !line.startsWith("task_id:"))
		.join("\n")
		.trim()
}

function patchLine(file) {
	if (!file || typeof file !== "object") return "patch"
	const rel = file.relativePath || file.filePath || ""
	if (file.type === "add") return `+ Created ${rel}`
	if (file.type === "delete") return `- Deleted ${rel}`
	if (file.type === "move") return `→ Moved ${file.filePath} -> ${rel}`
	return `~ Patched ${rel}`
}

function looksMarkdown(text) {
	return /^(#{1,6}\s|```|[-*]\s|\d+\.\s|>\s)/m.test(text) || /\*\*[^*]+\*\*|`[^`]+`/.test(text)
}

function looksDiff(text) {
	const lines = String(text)
		.split("\n")
		.filter((l) => l.trim())
	if (lines.length < 2) return false
	const hits = lines.filter((l) => /^[+-]/.test(l) || l.startsWith("@@")).length
	return hits / lines.length >= 0.5
}

function indentLines(text, indent) {
	return String(text)
		.split("\n")
		.map((l) => indent + l)
		.join("\n")
}

function titlecase(s) {
	return String(s).replace(/[_-]+/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())
}

function obj(v) {
	return v && typeof v === "object" && !Array.isArray(v) ? v : {}
}
