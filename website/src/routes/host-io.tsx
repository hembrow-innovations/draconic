import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/host-io")({
  loader: () => loadMarkdownPage("host-io"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: HostIoRoute,
});

/**
 * host I/O chapter. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
 */
function HostIoRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
