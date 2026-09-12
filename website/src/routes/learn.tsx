import { createFileRoute } from "@tanstack/react-router";
import { DocsShell } from "../features/docs/DocsShell";
import { LearnHubCards } from "../features/learn/LearnHubCards";
import { LearnNav } from "../features/learn/LearnNav";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

export const Route = createFileRoute("/learn")({
  loader: () => loadMarkdownPage("learn"),
  head: ({ loaderData }) =>
    pageShareHead({
      title: loaderData ? `${loaderData.title} · Draconic` : "Draconic",
      path: loaderData ? `/${loaderData.slug}` : "/learn",
      description: loaderData
        ? pageDescriptionFromBody(loaderData.body)
        : "",
    }),
  component: LearnHubRoute,
});

/**
 * Learn hub in handbook chrome. Locks `public-site.ia:learn-walkable`,
 * `public-site.chrome:document-title`, and `public-site.chrome:meta-description`.
 *
 * Teaching copy stays in `website/learn.md`.
 */
function LearnHubRoute() {
  const page = Route.useLoaderData();
  return (
    <DocsShell
      kicker={page.section}
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<LearnNav />}
      body={page.body}
    >
      <LearnHubCards />
    </DocsShell>
  );
}
