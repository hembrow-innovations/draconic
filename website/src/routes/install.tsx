import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/install")({
  loader: () => loadMarkdownPage("install"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: InstallRoute,
});

/**
 * Install chapter. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
 */
function InstallRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
