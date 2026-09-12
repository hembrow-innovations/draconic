import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/modules")({
  loader: () => loadMarkdownPage("modules"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/modules",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: ModulesRoute,
});

/**
 * modules chapter. Locks `public-site.ia:learn-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function ModulesRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
