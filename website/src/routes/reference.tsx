import { createFileRoute } from "@tanstack/react-router";
import { DocsShell } from "../features/docs/DocsShell";
import { ReferenceHubCards } from "../features/reference/ReferenceHubCards";
import { ReferenceNav } from "../features/reference/ReferenceNav";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/reference")({
  loader: () => loadMarkdownPage("reference"),
  component: ReferenceHubRoute,
});

/**
 * Reference hub in handbook chrome. Locks `public-site.ia:reference-walkable`.
 *
 * Teaching copy stays in `website/reference.md`.
 */
function ReferenceHubRoute() {
  const page = Route.useLoaderData();
  return (
    <DocsShell
      kicker={page.section}
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<ReferenceNav />}
      body={page.body}
    >
      <ReferenceHubCards />
    </DocsShell>
  );
}
