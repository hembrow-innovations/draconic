import {
  listMarkdownPages,
  uniqueHeadingId,
  unwrapMarkdownLinks,
  type MarkdownPage,
} from "../content";

/**
 * One public page in the title, heading, and teaching-body finder.
 *
 * Locks `public-site.search:titles-headings`.
 */
export type SearchEntry = {
  href: string;
  title: string;
  section: string;
  headings: string[];
  body: string;
};

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
    const text = unwrapMarkdownLinks(line.slice(hashes).trim());
    if (text !== "") {
      headings.push(text);
    }
  }
  return headings;
}

/**
 * Build the static title, heading, and teaching-body index from routed `website/content/*.md` pages.
 *
 * @returns Finder entries for existing Start routes
 */
export function buildSearchIndex(): SearchEntry[] {
  return listMarkdownPages().map(toSearchEntry);
}

/**
 * Find Learn or Reference pages whose title, heading, or teaching body contains the query.
 *
 * @param index - Static title, heading, and teaching-body entries
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
 * Page path, plus a heading fragment when the query hit a section not the title.
 *
 * @param entry - Indexed page
 * @param query - Visitor search text
 * @returns Start path, with `#slug` when the match is a heading
 */
export function searchHitHref(entry: SearchEntry, query: string): string {
  const needle = query.trim().toLowerCase();
  if (needle === "" || entry.title.toLowerCase().includes(needle)) {
    return entry.href;
  }
  const seen = new Set<string>();
  for (const heading of entry.headings) {
    const id = uniqueHeadingId(heading, seen);
    if (heading !== entry.title && heading.toLowerCase().includes(needle)) {
      return `${entry.href}#${id}`;
    }
  }
  return entry.href;
}

/**
 * Visible result label: Learn or Reference, page title, and a matching heading.
 *
 * @param entry - Indexed page
 * @param query - Visitor search text
 * @returns Label that tells the two packages pages apart
 */
export function searchHitLabel(entry: SearchEntry, query: string): string {
  const sectionLabel = sectionDisplayName(entry.section);
  const needle = query.trim().toLowerCase();
  if (needle === "" || entry.title.toLowerCase().includes(needle)) {
    return `${sectionLabel} · ${entry.title}`;
  }
  const heading = entry.headings.find((text) =>
    text.toLowerCase().includes(needle),
  );
  if (heading !== undefined && heading !== entry.title) {
    return `${sectionLabel} · ${entry.title} · ${heading}`;
  }
  return `${sectionLabel} · ${entry.title}`;
}

function toSearchEntry(page: MarkdownPage): SearchEntry {
  return {
    href: `/${page.slug}`,
    title: page.title,
    section: page.section,
    headings: extractHeadings(page.body),
    body: page.body,
  };
}

function sectionDisplayName(section: string): string {
  switch (section) {
    case "learn":
      return "Learn";
    case "reference":
      return "Reference";
    default:
      return section;
  }
}

function matchesEntry(entry: SearchEntry, needle: string): boolean {
  if (entry.title.toLowerCase().includes(needle)) {
    return true;
  }
  if (entry.headings.some((heading) => heading.toLowerCase().includes(needle))) {
    return true;
  }
  return entry.body.toLowerCase().includes(needle);
}
