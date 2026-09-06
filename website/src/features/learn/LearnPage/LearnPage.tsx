import { DocsShell } from "../../docs/DocsShell";
import { LearnNav } from "../LearnNav";
import type { LearnPageProps } from "./LearnPage.types";

/**
 * Learn article in handbook chrome. Locks `public-site.ia:learn-walkable`.
 *
 * Teaching copy stays in `website/*.md`. Status comes from frontmatter.
 *
 * @param props - Loaded markdown page
 * @returns Docs shell with Learn nav and body
 */
export function LearnPage({ page }: LearnPageProps) {
  return (
    <DocsShell
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<LearnNav />}
      body={page.body}
    />
  );
}
