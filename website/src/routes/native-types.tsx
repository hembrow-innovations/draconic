import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/native-types")({
  loader: () => loadMarkdownPage("native-types"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: NativeTypesRoute,
});

/**
 * native types chapter. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
 */
function NativeTypesRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
