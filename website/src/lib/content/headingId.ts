/**
 * Visible heading text with markdown link markers stripped.
 *
 * @param text - Heading line after the `#` marker
 * @returns Link labels kept; destinations dropped
 */
export function unwrapMarkdownLinks(text: string): string {
  return text.replace(/\[([^\]]+)\]\([^)]*\)/g, "$1");
}

/**
 * Stable URL fragment for a heading's visible text.
 *
 * @param text - Heading text after link markers are unwrapped
 * @returns Lowercase hyphenated slug, or `section` when nothing remains
 */
export function slugifyHeading(text: string): string {
  const slug = text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return slug === "" ? "section" : slug;
}

/**
 * Unique heading fragment on one page, matching render order.
 *
 * @param text - Heading text after link markers are unwrapped
 * @param seen - Slugs already emitted on this page
 * @returns A slug not yet in `seen`, then records it
 */
export function uniqueHeadingId(text: string, seen: Set<string>): string {
  const base = slugifyHeading(text);
  let id = base;
  let n = 2;
  while (seen.has(id)) {
    id = `${base}-${n}`;
    n += 1;
  }
  seen.add(id);
  return id;
}
