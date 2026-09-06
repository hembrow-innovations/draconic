import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/cli")({
  loader: () => loadMarkdownPage("cli"),
  component: CliRoute,
});

/**
 * CLI working page. Locks `public-site.ia:reference-walkable`.
 */
function CliRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
