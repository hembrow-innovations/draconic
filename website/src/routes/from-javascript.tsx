import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/from-javascript")({
  loader: () => loadMarkdownPage("from-javascript"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: FromJavascriptRoute,
});

/**
 * from JavaScript landing. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
 */
function FromJavascriptRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
