import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/dual-worlds")({
  loader: () => loadMarkdownPage("dual-worlds"),
  component: DualWorldsRoute,
});

/**
 * Dual worlds join. Locks `public-site.ia:learn-walkable`.
 */
function DualWorldsRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
