import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/from-systems")({
  loader: () => loadMarkdownPage("from-systems"),
  component: FromSystemsRoute,
});

/**
 * from systems landing. Locks `public-site.ia:learn-walkable`.
 */
function FromSystemsRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
