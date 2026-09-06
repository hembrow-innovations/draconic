import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/reference-host-io")({
  loader: () => loadMarkdownPage("reference-host-io"),
  component: ReferenceHostIoRoute,
});

/**
 * host I/O working page. Locks `public-site.ia:reference-walkable`.
 */
function ReferenceHostIoRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
