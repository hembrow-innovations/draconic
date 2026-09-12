import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/packages")({
  loader: () => loadMarkdownPage("packages"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: PackagesRoute,
});

/**
 * packages chapter. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
 */
function PackagesRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
