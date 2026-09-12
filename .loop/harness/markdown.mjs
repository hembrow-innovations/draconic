import { wrapAnsi } from "./ansi.mjs"

export function renderMarkdown(src, { paint, width = 80, indent = "  " } = {}) {
	if (!src) return ""
	const lines = String(src).replace(/\r\n/g, "\n").split("\n")
	const blocks = []
	let i = 0
	while (i < lines.length) {
		const line = lines[i]
		if (/^\s*$/.test(line)) {
			i++
			continue
		}
		const fence = line.match(/^\s*(```|~~~)\s*(.*)$/)
		if (fence) {
			const mark = fence[1]
			const lang = fence[2].trim()
			const body = []
			i++
			while (i < lines.length && !lines[i].trim().startsWith(mark)) {
				body.push(lines[i])
				i++
			}
			if (i < lines.length) i++
			blocks.push(renderFence(lang, body.join("\n"), paint, width, indent))
			continue
		}
		if (/^\s{0,3}#{1,6}\s+\S/.test(line)) {
			blocks.push(renderHeading(line, paint, width, indent))
			i++
			continue
		}
		if (/^\s{0,3}(?:-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
			const n = Math.min(32, Math.max(8, width - indent.length))
			blocks.push(indent + paint.dim("─".repeat(n)))
			i++
			continue
		}
		if (/^\s{0,3}>/.test(line)) {
			const q = []
			while (i < lines.length && /^\s{0,3}>/.test(lines[i])) {
				q.push(lines[i].replace(/^\s{0,3}>\s?/, ""))
				i++
			}
			blocks.push(renderQuote(q.join("\n"), paint, width, indent))
			continue
		}
		if (isTableStart(lines, i)) {
			const table = takeTable(lines, i)
			blocks.push(renderTable(table.rows, paint, indent))
			i = table.next
			continue
		}
		if (isListLine(line)) {
			const items = []
			while (i < lines.length) {
				const cur = lines[i]
				if (!cur.trim()) break
				if (isListLine(cur) || (items.length && isListCont(cur))) {
					items.push(cur)
					i++
					continue
				}
				break
			}
			blocks.push(renderList(items, paint, width, indent))
			continue
		}
		const para = []
		while (i < lines.length && lines[i].trim() && !isBlockStart(lines[i])) {
			para.push(lines[i].trim())
			i++
		}
		blocks.push(wrapAnsi(renderInline(para.join(" "), paint), width, indent))
	}
	return blocks.join("\n\n")
}

function isBlockStart(line) {
	return (
		/^\s*(```|~~~)/.test(line) ||
		/^\s{0,3}#{1,6}\s+\S/.test(line) ||
		/^\s{0,3}(?:-{3,}|\*{3,}|_{3,})\s*$/.test(line) ||
		/^\s{0,3}>/.test(line) ||
		isListLine(line)
	)
}

function isListLine(line) {
	return /^\s*(?:[-*+]|\d+[.)])\s+\S/.test(line)
}

function isListCont(line) {
	return /^\s{2,}\S/.test(line) && !isBlockStart(line)
}

function isTableStart(lines, i) {
	const a = lines[i]
	const b = lines[i + 1]
	if (!a || !b || !a.includes("|") || !b.includes("|")) return false
	return /^\s*\|?[\s:|-]+\|[\s:|-]+\|?\s*$/.test(b)
}

function takeTable(lines, i) {
	const rows = []
	let n = i
	while (n < lines.length && lines[n].includes("|") && lines[n].trim()) {
		rows.push(lines[n])
		n++
	}
	return { rows, next: n }
}

function renderHeading(line, paint, width, indent) {
	const m = line.match(/^\s{0,3}(#{1,6})\s+(.*)$/)
	const level = m[1].length
	const text = renderInline(m[2].replace(/\s+#+\s*$/, ""), paint)
	const style = level === 1 ? paint.h1 : level === 2 ? paint.h2 : paint.h3
	return wrapAnsi(style(text), width, indent)
}

function renderFence(lang, body, paint, width, indent) {
	const label = lang || "code"
	const bar = Math.max(4, Math.min(24, width - indent.length - label.length - 4))
	const head = indent + paint.dim(`── ${label} ${"─".repeat(bar)}`)
	const colored = colorFence(lang, body, paint)
		.split("\n")
		.map((l) => indent + "  " + l)
		.join("\n")
	return `${head}\n${colored}`
}

function colorFence(lang, body, paint) {
	if (lang === "diff" || lang === "patch" || looksDiff(body)) return renderDiff(body, paint)
	return String(body)
		.split("\n")
		.map((l) => paint.dim(l))
		.join("\n")
}

export function renderDiff(body, paint) {
	return String(body)
		.split("\n")
		.map((line) => {
			if (line.startsWith("+") && !line.startsWith("+++")) return paint.green(line)
			if (line.startsWith("-") && !line.startsWith("---")) return paint.red(line)
			if (line.startsWith("@@")) return paint.cyan(line)
			return paint.dim(line)
		})
		.join("\n")
}

function looksDiff(body) {
	const lines = String(body)
		.split("\n")
		.filter((l) => l.trim())
	if (lines.length < 2) return false
	const hits = lines.filter((l) => /^[+-]/.test(l) || l.startsWith("@@")).length
	return hits / lines.length >= 0.5
}

function renderQuote(text, paint, width, indent) {
	const inner = renderMarkdown(text, { paint, width: width - 2, indent: "" })
	return inner
		.split("\n")
		.map((l) => indent + paint.dim("│ ") + (l ? paint.italic(l) : ""))
		.join("\n")
}

function renderList(raw, paint, width, indent) {
	const out = []
	for (const line of raw) {
		const m = line.match(/^(\s*)([-*+]|\d+[.)])\s+(?:\[([ xX])\]\s+)?(.*)$/)
		if (!m) {
			out.push(wrapAnsi(renderInline(line.trim(), paint), width, indent + "    "))
			continue
		}
		const depth = Math.min(6, Math.floor(m[1].replace(/\t/g, "  ").length / 2))
		const pad = indent + "  ".repeat(depth)
		const check = m[3]
		const bullet =
			check != null
				? check.trim()
					? paint.ok("☑")
					: paint.dim("☐")
				: /^\d/.test(m[2])
					? paint.dim(m[2])
					: paint.dim("•")
		const mark = `${bullet} `
		out.push(wrapAnsi(renderInline(m[4], paint), width, pad + mark, pad + " ".repeat(stripLen(mark))))
	}
	return out.join("\n")
}

function stripLen(s) {
	return String(s).replace(/\x1b\[[0-9;]*m/g, "").length
}

function renderTable(rows, paint, indent) {
	const cells = rows
		.filter((row, i) => i !== 1)
		.map((row) =>
			row
				.replace(/^\s*\|/, "")
				.replace(/\|\s*$/, "")
				.split("|")
				.map((c) => c.trim()),
		)
	if (!cells.length) return ""
	const cols = Math.max(...cells.map((r) => r.length))
	const widths = Array.from({ length: cols }, (_, i) =>
		Math.min(40, Math.max(...cells.map((r) => (r[i] ? r[i].length : 0)))),
	)
	return cells
		.map((row, ri) => {
			const line = row
				.map((c, i) => (c || "").padEnd(widths[i]))
				.join(paint.dim(" │ "))
			const styled = ri === 0 ? paint.bold(line) : line
			return indent + styled
		})
		.join("\n")
}

export function renderInline(src, paint) {
	const s = String(src)
	let i = 0
	let out = ""
	while (i < s.length) {
		if (s[i] === "\\" && i + 1 < s.length) {
			out += s[i + 1]
			i += 2
			continue
		}
		if (s[i] === "`") {
			const end = s.indexOf("`", i + 1)
			if (end !== -1) {
				out += paint.code(s.slice(i + 1, end))
				i = end + 1
				continue
			}
		}
		if (s.startsWith("![", i)) {
			const m = s.slice(i).match(/^!\[([^\]]*)\]\(([^)]+)\)/)
			if (m) {
				out += paint.dim(`[image: ${m[1] || m[2]}]`)
				i += m[0].length
				continue
			}
		}
		if (s[i] === "[") {
			const m = s.slice(i).match(/^\[([^\]]+)\]\(([^)]+)\)/)
			if (m) {
				out += m[1] === m[2] ? paint.link(m[1]) : `${paint.link(m[1])}${paint.dim(` (${m[2]})`)}`
				i += m[0].length
				continue
			}
		}
		if (s.startsWith("***", i)) {
			const end = s.indexOf("***", i + 3)
			if (end !== -1) {
				out += paint.bold(paint.italic(s.slice(i + 3, end)))
				i = end + 3
				continue
			}
		}
		if (s.startsWith("**", i)) {
			const end = s.indexOf("**", i + 2)
			if (end !== -1) {
				out += paint.bold(renderInline(s.slice(i + 2, end), paint))
				i = end + 2
				continue
			}
		}
		if (s.startsWith("~~", i)) {
			const end = s.indexOf("~~", i + 2)
			if (end !== -1) {
				out += paint.strike(s.slice(i + 2, end))
				i = end + 2
				continue
			}
		}
		if (s[i] === "*" || (s[i] === "_" && (i === 0 || /[\s(]/.test(s[i - 1])))) {
			const mark = s[i]
			const end = s.indexOf(mark, i + 1)
			if (end !== -1 && end > i + 1 && (mark !== "_" || end === s.length - 1 || /[\s).,!?]/.test(s[end + 1] || " "))) {
				out += paint.italic(s.slice(i + 1, end))
				i = end + 1
				continue
			}
		}
		const url = s.slice(i).match(/^https?:\/\/[^\s)<]+/)
		if (url) {
			out += paint.link(url[0].replace(/[.,;:!?]+$/, ""))
			i += url[0].length
			continue
		}
		out += s[i]
		i++
	}
	return out
}
