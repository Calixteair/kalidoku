import type { CapacitorConfig } from "@capacitor/cli";

// Capacitor scaffolding kept ready for phase 2 (mobile native apps).
// `npm-build` outputs to dist/, which Capacitor wraps as the webDir.
const config: CapacitorConfig = {
  appId: "fr.calixteair.kalidoku",
  appName: "kalidoku",
  webDir: "dist",
  bundledWebRuntime: false,
  server: {
    androidScheme: "https",
  },
};

export default config;
