import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E configuration.
 *
 * Tests require the Rust server running at 127.0.0.1:8080 with
 * --enable-reset so the /reset endpoint is available for test isolation.
 * The Vite dev server is started automatically by the `webServer` block.
 *
 * Run:
 *   # Terminal 1 — Rust server (must already be running)
 *   cargo run --features http --bin serve -- --enable-reset
 *   # Terminal 2 — E2E tests
 *   npm run test:e2e
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false, // tests share a live server — run sequentially
  retries: 1,
  timeout: 15_000,
  use: {
    baseURL: "http://localhost:5173",
    // Headless by default; set PWDEBUG=1 to open the browser.
    headless: true,
    // Capture screenshots on failure for debugging.
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  // Start Vite dev server automatically before running tests.
  webServer: {
    command: "npm run dev",
    url: "http://localhost:5173",
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
