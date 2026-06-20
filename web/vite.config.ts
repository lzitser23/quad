import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// base: "./" makes built asset URLs relative, so they resolve under Tauri's asset protocol.
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    chunkSizeWarningLimit: 1500,
  },
});
