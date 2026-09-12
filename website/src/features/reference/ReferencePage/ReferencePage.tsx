import { DocsShell } from "../../docs/DocsShell";
import { ReferenceNav } from "../ReferenceNav";
import { ReferencePager } from "../ReferencePager";
import type { ReferencePageProps } from "./ReferencePage.types";

/**
 * Reference article in handbook chrome. Locks `public-site.ia:reference-walkable`
 * and `public-site.chrome:docs-article-order`.
 *
 * Teaching copy stays in `website/content/*.md`. Status comes from frontmatter.
 *
 * @param props - Loaded markdown page
 * @returns Docs shell with Reference nav, body, and prev/next
 */
export function ReferencePage({ page }: ReferencePageProps) {
  return (
    <DocsShell
      kicker={page.section}
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<ReferenceNav />}
      body={page.body}
    >
      <ReferencePager slug={page.slug} />
    </DocsShell>
  );
}
