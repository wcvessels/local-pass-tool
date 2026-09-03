import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    hmr: false,
  },
  build: {
    outDir: "dist-ui",
    modulePreload: {
      polyfill: false,
    },
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
  },
});
