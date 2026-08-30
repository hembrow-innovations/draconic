export type VerdictResult = { ok: boolean; text: string };

const KINDS = ["TASK", "TICKET", "ESCALATE", "VERIFY"] as const;

export function recordVerdict(input: {
	kind: string;
	evidence: string;
}): VerdictResult {
	if (!KINDS.includes(input.kind as (typeof KINDS)[number])) {
		return {
			ok: false,
			text: "target must be TASK, TICKET, ESCALATE, or VERIFY",
		};
	}
	const evidence = input.evidence.trim();
	if (!evidence) {
		return { ok: false, text: "evidence is required for action verdict" };
	}
	const line = evidence.split(/\r?\n/, 1)[0]?.trim() ?? "";
	if (!line) {
		return { ok: false, text: "evidence is required for action verdict" };
	}
	return {
		ok: true,
		text: `VERDICT: ${input.kind}\nEVIDENCE: ${line}`,
	};
}
