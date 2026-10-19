// The parts of the Clarity UI library on the web, run against the cases
// every renderer shares (conformance/*.json), in a simulated browser page.
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  test: {
    environment: "jsdom",
    include: ["src/lib/ui/**/*.svelte.test.ts"],
  },
});
