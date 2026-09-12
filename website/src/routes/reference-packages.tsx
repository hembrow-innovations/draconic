import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/reference-packages")({
  loader: () => loadMarkdownPage("reference-packages"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/reference-packages",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: ReferencePackagesRoute,
});

/**
 * packages working page. Locks `public-site.ia:reference-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function ReferencePackagesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
