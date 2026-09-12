import { createFileRoute } from "@tanstack/react-router";
import { ReferencePage } from "../features/reference/ReferencePage";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/dual-world-rules")({
  loader: () => loadMarkdownPage("dual-world-rules"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/dual-world-rules",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: DualWorldRulesRoute,
});

/**
 * Dual-world rules working page. Locks `public-site.ia:reference-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 */
function DualWorldRulesRoute() {
  const page = Route.useLoaderData();
  return <ReferencePage page={page} />;
}
