import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// base: "./" makes built asset URLs relative, so they resolve correctly under
// WebView2's virtual host (https://winrect.local/index.html).
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    chunkSizeWarningLimit: 1500,
  },
});
