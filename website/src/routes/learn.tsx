import { createFileRoute } from "@tanstack/react-router";
import { DocsShell } from "../features/docs/DocsShell";
import { LearnHubCards } from "../features/learn/LearnHubCards";
import { LearnNav } from "../features/learn/LearnNav";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/learn")({
  loader: () => loadMarkdownPage("learn"),
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.title} · Draconic`
          : "Draconic",
      },
    ],
  }),
  component: LearnHubRoute,
});

/**
 * Learn hub in handbook chrome. Locks `public-site.ia:learn-walkable`
 * and `public-site.chrome:document-title`.
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
