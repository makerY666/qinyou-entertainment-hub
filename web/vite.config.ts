import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5174,
    strictPort: true,
    proxy: {
      "/api": "http://127.0.0.1:18765",
      "/ws": { target: "ws://127.0.0.1:18765", ws: true },
    },
  },
  build: { target: "es2020" },
});
