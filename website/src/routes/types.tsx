import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/types")({
  loader: () => loadMarkdownPage("types"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: TypesRoute,
});

/**
 * types working page. Locks `public-site.ia:reference-walkable`
 * and `public-site.chrome:document-title`.
 */
function TypesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
