import { ARCHIVE_KEYS } from "./archive.ts";
import { PLANNING_KEYS } from "./planning.ts";
import { QUARANTINE_KEYS } from "./quarantine.ts";
import { TICKET_KEYS } from "./ticket.ts";

export function namedAllowlist(name: string): ReadonlySet<string> {
  if (name === "ticket") return TICKET_KEYS;
  if (name === "planning") return PLANNING_KEYS;
  if (name === "archive") return ARCHIVE_KEYS;
  if (name === "quarantine") return QUARANTINE_KEYS;
  throw new Error(`Unknown folder schema "${name}"`);
}
