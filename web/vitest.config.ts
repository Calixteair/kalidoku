import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
    exclude: ["tests/e2e/**", "node_modules/**", "dist/**", ".astro/**"],
    globals: false,
    passWithNoTests: true,
  },
  resolve: {
    alias: {
      $lib: "/src/lib",
      $paraglide: "/src/paraglide",
    },
  },
});
