import { uniqueHeadingId, unwrapMarkdownLinks } from "./headingId";

/**
 * One section heading in an article outline, with the same fragment id the renderer emits.
 */
export type PageOutlineItem = {
  id: string;
  text: string;
  level: 2 | 3;
};

/**
 * Section headings below the title, in document order, skipping fenced samples.
 *
 * Ids match `renderMarkdown` permalinks so in-article outline links land on the heading.
 *
 * @param source - Markdown body without frontmatter
 * @returns Outline items for h2 and h3 headings
 */
export function extractPageOutline(source: string): PageOutlineItem[] {
  const items: PageOutlineItem[] = [];
  const seen = new Set<string>();
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
    const text = line.slice(hashes).startsWith(" ")
      ? line.slice(hashes + 1)
      : line.slice(hashes);
    const visible = unwrapMarkdownLinks(text);
    if (visible === "") {
      continue;
    }
    const id = uniqueHeadingId(visible, seen);
    const level = hashes <= 3 ? hashes : 4;
    if (level === 2 || level === 3) {
      items.push({ id, text: visible, level });
    }
  }

  return items;
}
