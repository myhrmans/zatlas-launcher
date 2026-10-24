import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// Tauri loads the dev server on a fixed port, and the built files from dist/ (src-tauri/tauri.conf.json)
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { target: "chrome120", outDir: "dist" },
});
