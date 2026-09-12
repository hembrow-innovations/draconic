import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/from-javascript")({
  loader: () => loadMarkdownPage("from-javascript"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/from-javascript",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: FromJavascriptRoute,
});

/**
 * from JavaScript landing. Locks `public-site.ia:learn-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function FromJavascriptRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
