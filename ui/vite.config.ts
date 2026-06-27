import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Tauri expects a fixed port; fail if it isn't available.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],

  // Vite options tailored for Tauri development.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 1421 }
      : undefined,
    watch: {
      // Tell vite to ignore watching `src-tauri` so the Rust change
      // watcher doesn't trigger a Vite reload.
      ignored: ['**/src-tauri/**'],
    },
  },

  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test-setup.ts'],
    css: false,
  },
});
