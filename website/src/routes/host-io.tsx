import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/host-io")({
  loader: () => loadMarkdownPage("host-io"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/host-io",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: HostIoRoute,
});

/**
 * host I/O chapter. Locks `public-site.ia:learn-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function HostIoRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
