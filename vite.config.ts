import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Tauri expects a fixed port and ignores VITE_ env vars at build time.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tauri's Rust sources should not trigger frontend HMR.
      ignored: ["**/src-tauri/**"],
    },
  },

  // Produce smaller, predictable output for the desktop bundle.
  build: {
    target: "es2022",
    minify: "esbuild",
    sourcemap: false,
  },
}));
