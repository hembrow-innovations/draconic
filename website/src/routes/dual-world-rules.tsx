import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/dual-world-rules")({
  loader: () => loadMarkdownPage("dual-world-rules"),
  component: DualWorldRulesRoute,
});

/**
 * Dual-world rules working page. Locks `public-site.ia:reference-walkable`.
 */
function DualWorldRulesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
