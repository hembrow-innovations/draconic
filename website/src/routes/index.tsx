import { createFileRoute } from "@tanstack/react-router";
import { HomeFeatures } from "../features/home/HomeFeatures";
import { HomeHero } from "../features/home/HomeHero";
import { HomeSample } from "../features/home/HomeSample";

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
      <HomeSample />
      <HomeFeatures />
    </>
  );
}
