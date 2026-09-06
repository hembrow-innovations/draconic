import { createFileRoute } from "@tanstack/react-router";
import { DocsShell } from "../features/docs/DocsShell";
import { LearnNav } from "../features/learn/LearnNav";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/learn")({
  loader: () => loadMarkdownPage("learn"),
  component: LearnHubRoute,
});

/**
 * Learn hub in handbook chrome. Locks `public-site.ia:learn-walkable`.
 *
 * Teaching copy stays in `website/learn.md`. Chapter bodies wait for a later sitting.
 */
function LearnHubRoute() {
  const page = Route.useLoaderData();
  return (
    <DocsShell
      status={page.status === "not-yet" ? "not-yet" : "shipped"}
      nav={<LearnNav />}
      body={page.body}
    />
  );
}
