import { DocsShell } from "../../docs/DocsShell";
import { LearnNav } from "../LearnNav";
import { LearnPager } from "../LearnPager";
import type { LearnPageProps } from "./LearnPage.types";

/**
 * Learn article in handbook chrome. Locks `public-site.ia:learn-walkable`.
 *
 * Teaching copy stays in `website/*.md`. Status comes from frontmatter.
 *
 * @param props - Loaded markdown page
 * @returns Docs shell with Learn nav, body, and prev/next
 */
export function LearnPage({ page }: LearnPageProps) {
  return (
    <DocsShell
      kicker={page.section}
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<LearnNav />}
      body={page.body}
    >
      <LearnPager slug={page.slug} />
    </DocsShell>
  );
}
