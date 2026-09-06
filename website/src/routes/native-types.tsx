import { createFileRoute } from "@tanstack/react-router";
import { LearnPage } from "../features/learn/LearnPage";
import { loadMarkdownPage } from "../lib/content";

export const Route = createFileRoute("/native-types")({
  loader: () => loadMarkdownPage("native-types"),
  component: NativeTypesRoute,
});

/**
 * native types chapter. Locks `public-site.ia:learn-walkable`.
 */
function NativeTypesRoute() {
  const page = Route.useLoaderData();
  return <LearnPage page={page} />;
}
