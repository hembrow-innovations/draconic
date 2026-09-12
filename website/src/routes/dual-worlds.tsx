import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/dual-worlds")({
  loader: () => loadMarkdownPage("dual-worlds"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/dual-worlds",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: DualWorldsRoute,
});

/**
 * Dual worlds join. Locks `public-site.ia:learn-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function DualWorldsRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
