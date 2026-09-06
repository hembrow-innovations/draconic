import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/reference-packages")({
  loader: () => loadMarkdownPage("reference-packages"),
  component: ReferencePackagesRoute,
});

/**
 * packages working page. Locks `public-site.ia:reference-walkable`.
 */
function ReferencePackagesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
