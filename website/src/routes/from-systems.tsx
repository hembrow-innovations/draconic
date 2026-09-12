import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/from-systems")({
  loader: () => loadMarkdownPage("from-systems"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: FromSystemsRoute,
});

/**
 * from systems landing. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
 */
function FromSystemsRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
