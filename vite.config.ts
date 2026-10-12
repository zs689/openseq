import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri drives this dev server on a fixed port; see src-tauri/tauri.conf.json.
export default defineConfig({
  plugins: [react()],
  // Relative asset URLs: keeps the bundle openable from file:// as well as
  // from Tauri's custom protocol.
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
