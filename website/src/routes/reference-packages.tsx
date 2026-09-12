import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/reference-packages")({
  loader: () => loadMarkdownPage("reference-packages"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: ReferencePackagesRoute,
});

/**
 * packages working page. Locks `public-site.ia:reference-walkable`
 * and `public-site.chrome:document-title`.
 */
function ReferencePackagesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
