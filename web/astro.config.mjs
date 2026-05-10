import { defineConfig } from "astro/config";
import svelte from "@astrojs/svelte";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://kalidoku.calixteair.fr",
  output: "static",
  trailingSlash: "never",
  build: { format: "directory" },
  // /play used to fall back to index.html via the nginx SPA rewrite, surfacing
  // a phantom duplicate of the home page. Redirect until the dedicated
  // solo-mode page lands in phase 2.
  redirects: {
    "/play": "/",
  },
  i18n: {
    defaultLocale: "fr",
    locales: ["fr", "en"],
    routing: {
      prefixDefaultLocale: false,
      redirectToDefaultLocale: false,
    },
  },
  integrations: [svelte()],
  vite: {
    plugins: [tailwindcss()],
    server: {
      proxy: {
        "/api": { target: "http://localhost:8080", changeOrigin: true },
      },
    },
  },
});
