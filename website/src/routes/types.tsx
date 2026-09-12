import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/types")({
  loader: () => loadMarkdownPage("types"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/types",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: TypesRoute,
});

/**
 * types working page. Locks `public-site.ia:reference-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function TypesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
