import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/types")({
  loader: () => loadMarkdownPage("types"),
  component: TypesRoute,
});

/**
 * types working page. Locks `public-site.ia:reference-walkable`.
 */
function TypesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
