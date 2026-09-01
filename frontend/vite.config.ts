import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";
import { reviewApi } from "./review/plugin.ts";

export default defineConfig(({ mode }) => ({
  plugins: [vue(), tailwindcss(), ...(mode === "review" ? [reviewApi()] : [])],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    assetsDir: "assets",
    sourcemap: false,
  },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8080",
      "/auth": "http://127.0.0.1:8080",
      "/openapi.json": "http://127.0.0.1:8080",
    },
  },
}));
