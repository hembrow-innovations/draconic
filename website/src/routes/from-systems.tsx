import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/from-systems")({
  loader: () => loadMarkdownPage("from-systems"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/from-systems",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: FromSystemsRoute,
});

/**
 * from systems landing. Locks `public-site.ia:learn-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function FromSystemsRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
