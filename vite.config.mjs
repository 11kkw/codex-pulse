import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  build: {
    outDir: "dist/client",
  },
  optimizeDeps: {
    include: ["react", "react-dom/client"],
  },
  server: {
    host: "0.0.0.0",
    allowedHosts: ["terminal.local"],
    port: 1420,
    warmup: {
      clientFiles: ["./src/main.tsx"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  plugins: [react()],
});
