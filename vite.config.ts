import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

interface ProcessEnv {
  TAURI_DEV_HOST?: string
}

declare const process: {
  env: ProcessEnv
}

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    host: host || false,
    ...(host
      ? {
          hmr: {
            host,
            port: 1421,
            protocol: 'ws',
          },
        }
      : {}),
    port: 1420,
    strictPort: true,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}));
