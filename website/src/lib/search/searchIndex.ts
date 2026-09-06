import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { listMarkdownPages, type MarkdownPage } from "../content";

/**
 * One public page in the title and heading finder.
 *
 * Locks `public-site.search:titles-headings`.
 */
export type SearchEntry = {
  href: string;
  title: string;
  headings: string[];
};

const routesDir = join(dirname(fileURLToPath(import.meta.url)), "../../routes");

/**
 * Pull heading lines from a teaching-page body, skipping fenced samples.
 *
 * @param source - Markdown body without frontmatter
 * @returns Visible heading text in document order
 */
export function extractHeadings(source: string): string[] {
  const headings: string[] = [];
  let inFence = false;
  for (const line of source.split(/\r?\n/)) {
    if (line.startsWith("```")) {
      inFence = !inFence;
      continue;
    }
    if (inFence || !line.startsWith("#")) {
      continue;
    }
    let hashes = 0;
    while (hashes < line.length && line[hashes] === "#") {
      hashes += 1;
    }
    const text = unwrapLinks(line.slice(hashes).trim());
    if (text !== "") {
      headings.push(text);
    }
  }
  return headings;
}

/**
 * Build the static title and heading index from routed `website/*.md` pages.
 *
 * @returns Finder entries for existing Start routes
 */
export function buildSearchIndex(): SearchEntry[] {
  return listMarkdownPages()
    .filter((page) => existsSync(join(routesDir, `${page.slug}.tsx`)))
    .map(toSearchEntry);
}

/**
 * Find Learn or Reference pages whose title or heading contains the query.
 *
 * @param index - Static title and heading entries
 * @param query - Visitor search text
 * @returns Matching pages in index order
 */
export function querySearchIndex(
  index: readonly SearchEntry[],
  query: string,
): SearchEntry[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") {
    return [];
  }
  return index.filter((entry) => matchesEntry(entry, needle));
}

/**
 * Map a loaded teaching page to a finder entry.
 *
 * @param page - Parsed `website/*.md` page with a Start route
 * @returns Title, headings, and app href
 */
function toSearchEntry(page: MarkdownPage): SearchEntry {
  return {
    href: `/${page.slug}`,
    title: page.title,
    headings: extractHeadings(page.body),
  };
}

/**
 * Whether a page title or heading contains the needle.
 *
 * @param entry - One finder entry
 * @param needle - Lowercased query
 * @returns True when title or a heading matches
 */
function matchesEntry(entry: SearchEntry, needle: string): boolean {
  if (entry.title.toLowerCase().includes(needle)) {
    return true;
  }
  return entry.headings.some((heading) => heading.toLowerCase().includes(needle));
}

/**
 * Use link labels as the visible heading text.
 *
 * @param text - One heading line
 * @returns Heading text without markdown link markup
 */
function unwrapLinks(text: string): string {
  return text.replace(/\[([^\]]+)\]\([^)]*\)/g, "$1");
}
