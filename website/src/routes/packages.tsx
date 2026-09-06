import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/packages")({
  loader: () => loadMarkdownPage("packages"),
  component: PackagesRoute,
});

/**
 * packages chapter. Locks `public-site.ia:learn-walkable`.
 */
function PackagesRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
