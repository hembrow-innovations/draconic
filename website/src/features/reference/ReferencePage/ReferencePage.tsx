import { DocsShell } from "../../docs/DocsShell";
import { ReferenceNav } from "../ReferenceNav";
import type { ReferencePageProps } from "./ReferencePage.types";

/**
 * Reference article in handbook chrome. Locks `public-site.ia:reference-walkable`.
 *
 * Teaching copy stays in `website/*.md`. Status comes from frontmatter.
 *
 * @param props - Loaded markdown page
 * @returns Docs shell with Reference nav and body
 */
export function ReferencePage({ page }: ReferencePageProps) {
  return (
    <DocsShell
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<ReferenceNav />}
      body={page.body}
    />
  );
}
