const RESET = "\x1b[0m"

export function createPaint(on) {
	const seq = (open) => (s) => {
		const t = String(s ?? "")
		return on ? `${open}${t}${RESET}` : t
	}
	return {
		enabled: on,
		reset: RESET,
		bold: seq("\x1b[1m"),
		dim: seq("\x1b[2m"),
		italic: seq("\x1b[3m"),
		under: seq("\x1b[4m"),
		strike: seq("\x1b[9m"),
		red: seq("\x1b[31m"),
		green: seq("\x1b[32m"),
		yellow: seq("\x1b[33m"),
		blue: seq("\x1b[34m"),
		magenta: seq("\x1b[35m"),
		cyan: seq("\x1b[36m"),
		gray: seq("\x1b[90m"),
		h1: seq("\x1b[1;97m"),
		h2: seq("\x1b[1;36m"),
		h3: seq("\x1b[1;34m"),
		code: seq("\x1b[33m"),
		link: seq("\x1b[4;36m"),
		ok: seq("\x1b[32m"),
		err: seq("\x1b[31m"),
		think: seq("\x1b[2;3;35m"),
	}
}

export function stripAnsi(s) {
	return String(s).replace(/\x1b\[[0-9;]*m/g, "")
}

export function wrapAnsi(text, width, indent = "", hang = indent) {
	const chunks = String(text).split("\n")
	const out = []
	for (const chunk of chunks) {
		if (!chunk) {
			out.push("")
			continue
		}
		const tokens = chunk.match(/\x1b\[[0-9;]*m|[^\x1b\s]+|\s+/g) || []
		let line = ""
		let vis = 0
		let prefix = indent
		const maxFor = () => Math.max(16, width - prefix.length)
		const flush = () => {
			out.push(prefix + line.trimEnd())
			line = ""
			vis = 0
			prefix = hang
		}
		for (const tok of tokens) {
			if (tok.startsWith("\x1b")) {
				line += tok
				continue
			}
			const max = maxFor()
			if (/^\s+$/.test(tok)) {
				if (vis === 0) continue
				if (vis + tok.length > max) {
					flush()
					continue
				}
				line += tok
				vis += tok.length
				continue
			}
			if (vis && vis + tok.length > max) flush()
			const room = maxFor()
			if (tok.length > room && vis === 0) {
				let rest = tok
				while (rest.length > maxFor()) {
					const n = maxFor()
					line += rest.slice(0, n)
					vis = n
					flush()
					rest = rest.slice(n)
				}
				line += rest
				vis = rest.length
				continue
			}
			line += tok
			vis += tok.length
		}
		if (line || vis) flush()
		else if (!tokens.length) out.push(prefix.trimEnd())
	}
	return out.join("\n")
}

export function clipLines(text, max = 80) {
	const lines = String(text).split("\n")
	if (lines.length <= max) return String(text)
	return `${lines.slice(0, max).join("\n")}\n… ${lines.length - max} more lines`
}
