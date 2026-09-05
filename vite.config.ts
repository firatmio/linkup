import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

// Tüm instance'lar tek Vite sunucusundan beslenir (PLAN.md §6): profil ayrımı
// Rust tarafında --profile ile yapılır, frontend aynıdır.
const port = 1420;

export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],

  // Vite options tailored for Tauri development
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: port + 1 } : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
