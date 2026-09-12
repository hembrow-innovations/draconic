import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/dual-world-rules")({
  loader: () => loadMarkdownPage("dual-world-rules"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: DualWorldRulesRoute,
});

/**
 * Dual-world rules working page. Locks `public-site.ia:reference-walkable`
 * and `public-site.chrome:document-title`.
 */
function DualWorldRulesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
