import { createFileRoute } from "@tanstack/react-router";
import { DocsShell } from "../features/docs/DocsShell";
import { ReferenceHubCards } from "../features/reference/ReferenceHubCards";
import { ReferenceNav } from "../features/reference/ReferenceNav";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/reference")({
  loader: () => loadMarkdownPage("reference"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/reference",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: ReferenceHubRoute,
});

/**
 * Reference hub in handbook chrome. Locks `public-site.ia:reference-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 *
 * Teaching copy stays in `website/reference.md`.
 */
function ReferenceHubRoute() {
  const page = Route.useLoaderData();
  return (
    <DocsShell
      kicker={page.section}
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<ReferenceNav />}
      body={page.body}
    >
      <ReferenceHubCards />
    </DocsShell>
  );
}
