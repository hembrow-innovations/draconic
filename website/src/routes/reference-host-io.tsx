import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/reference-host-io")({
  loader: () => loadMarkdownPage("reference-host-io"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/reference-host-io",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: ReferenceHostIoRoute,
});

/**
 * host I/O working page. Locks `public-site.ia:reference-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function ReferenceHostIoRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
