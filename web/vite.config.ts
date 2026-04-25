import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Proxy /api/* to the Rust serve binary so the demo runs same-origin
// (no CORS / cookie issues) when both run locally.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8080",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ""),
      },
    },
  },
  test: {
    environment: "node",
    globals: false,
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/**/*.ts"],
      exclude: ["src/**/*.test.ts", "src/main.tsx", "src/components/**"],
    },
  },
});
