import { createFileRoute } from "@tanstack/react-router";
import { HomeFeatures } from "../features/home/HomeFeatures";
import { HomeHero } from "../features/home/HomeHero";

export const Route = createFileRoute("/")({
  component: HomeRoute,
});

/**
 * Language homepage at `/`, not Learn copied to index.
 */
function HomeRoute() {
  return (
    <>
      <HomeHero />
      <HomeFeatures />
    </>
  );
}
