import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/modules")({
  loader: () => loadMarkdownPage("modules"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: ModulesRoute,
});

/**
 * modules chapter. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
 */
function ModulesRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
