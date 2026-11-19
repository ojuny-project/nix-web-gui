import { defineConfig, loadEnv } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const page = (name: string) =>
  new URL(`./src/pages/${name}`, import.meta.url).pathname;

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, ".", "VITE_");
  const withOjunyWidgets = env.VITE_OJUNY_WIDGETS !== "0";
  return {
    plugins: [svelte()],
    resolve: {
      alias: withOjunyWidgets
        ? {
            "@ojuny/env-editor": page("EnvEditor.svelte"),
            "@ojuny/plugin-settings": page("PluginSettings.svelte"),
          }
        : {
            "@ojuny/env-editor": page("OjunyUnavailable.svelte"),
            "@ojuny/plugin-settings": page("OjunyUnavailable.svelte"),
          },
    },
    server: {
      proxy: {
        "/api": "http://127.0.0.1:8787",
      },
    },
    optimizeDeps: {
      exclude: ["@nix-web-gui/ui"],
    },
    build: {
      outDir: "dist",
      emptyOutDir: true,
    },
  };
});
