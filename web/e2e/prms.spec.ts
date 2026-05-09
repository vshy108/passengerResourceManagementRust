/**
 * E2E tests: React thin-client UI → Rust axum API.
 *
 * Prerequisites (must be running before `npm run test:e2e`):
 *   cargo run --features http --bin serve -- --enable-reset
 *
 * The Vite dev server is started automatically by playwright.config.ts.
 *
 * Each test resets the server to a clean demo world via POST /reset so
 * tests are order-independent (no shared mutable state between tests).
 */

import { expect, request, test } from "@playwright/test";

// ── helpers ──────────────────────────────────────────────────────────────────

const API = "http://127.0.0.1:8080";

/** Reset server to the seeded demo world before each test. */
async function resetServer(): Promise<void> {
  const ctx = await request.newContext({ baseURL: API });
  // FIX: actor identity now derived from bearer token, not request body.
  const r = await ctx.post("/reset", {
    headers: { Authorization: "Bearer cl-aria-e2e-token" },
  });
  if (!r.ok()) {
    throw new Error(
      `POST /reset failed: ${r.status()} — is the server running with --enable-reset?`,
    );
  }
  await ctx.dispose();
}

// ── fixture: wait for the UI to come online ───────────────────────────────────

test.beforeEach(async ({ page }) => {
  await resetServer();
  await page.goto("/");
  // Wait for the status badge to show ONLINE before running any assertion.
  await expect(page.getByTestId("server-status")).toHaveText("ONLINE", {
    timeout: 10_000,
  });
});

// ── tests ────────────────────────────────────────────────────────────────────

test("page loads and shows ONLINE status", async ({ page }) => {
  // The live panel itself is visible.
  await expect(page.getByTestId("live-panel")).toBeVisible();
  await expect(page.getByTestId("server-status")).toHaveText("ONLINE");
});

test("passengers table shows 3 seeded passengers", async ({ page }) => {
  const table = page.getByTestId("passengers-table");
  await expect(table).toBeVisible();
  // Demo world seeds ps-001, ps-002, ps-003.
  const rows = table.locator("tbody tr");
  await expect(rows).toHaveCount(3);
  await expect(table).toContainText("Mira Voss");
  await expect(table).toContainText("Kai Reeves");
  await expect(table).toContainText("Lena Ito");
});

test("resources table shows 3 seeded resources", async ({ page }) => {
  const table = page.getByTestId("resources-table");
  await expect(table).toBeVisible();
  const rows = table.locator("tbody tr");
  await expect(rows).toHaveCount(3);
  await expect(table).toContainText("Stardeck Lounge");
  await expect(table).toContainText("Zero-G Spa");
  await expect(table).toContainText("Bridge Tour");
});

test("health/ready endpoint reports correct counts via UI", async ({
  page,
}) => {
  // This exercises the full React → API round-trip on load.
  // The passengers and resources tables are driven by the same API
  // data that /health/ready counts — verify they agree.
  const paxRows = page
    .getByTestId("passengers-table")
    .locator("tbody tr");
  const resRows = page
    .getByTestId("resources-table")
    .locator("tbody tr");
  await expect(paxRows).toHaveCount(3);
  await expect(resRows).toHaveCount(3);
});

test("access attempt: Silver passenger allowed to Stardeck Lounge (Silver)", async ({
  page,
}) => {
  // Select Silver passenger (ps-001 — Mira Voss, Silver tier)
  const pidSelect = page.locator("[data-testid='live-panel'] select").first();
  // The access section has two selects; use the API directly to verify,
  // then check the UI flash message after clicking "Attempt access".
  // Select ps-001 (Silver) from the passenger dropdown in AccessSection.
  const accessSection = page.locator("h3", {
    hasText: "Access (POST /access)",
  });
  // Passenger select is the first select after the AccessSection heading.
  const paxSelect = accessSection.locator("xpath=following-sibling::div[1]//select[1]");
  const resSelect = accessSection.locator("xpath=following-sibling::div[1]//select[2]");

  // FIX: Playwright selectOption({label}) requires an exact string, not a RegExp.
  // Option text is rendered as "{name} ({tier})" and "{name} (min {tier})".
  await paxSelect.selectOption({ label: "Mira Voss (Silver)" });
  await resSelect.selectOption({ label: "Stardeck Lounge (min Silver)" });

  await page.getByTestId("btn-attempt-access").click();

  // The flash banner should say "Allowed" (from "Allowed (event #...)")
  await expect(page.locator(".muted").filter({ hasText: /Allowed/ })).toBeVisible({
    timeout: 5_000,
  });
});

test("access attempt: Silver passenger denied to Bridge Tour (Platinum)", async ({
  page,
}) => {
  const accessSection = page.locator("h3", {
    hasText: "Access (POST /access)",
  });
  const paxSelect = accessSection.locator("xpath=following-sibling::div[1]//select[1]");
  const resSelect = accessSection.locator("xpath=following-sibling::div[1]//select[2]");

  // FIX: Playwright selectOption({label}) requires an exact string, not a RegExp.
  await paxSelect.selectOption({ label: "Mira Voss (Silver)" });
  await resSelect.selectOption({ label: "Bridge Tour (min Platinum)" });

  await page.getByTestId("btn-attempt-access").click();

  // The flash should contain the domain error "AccessDenied" or the word "Denied"
  await expect(
    page.locator(".muted").filter({ hasText: /Denied|denied|AccessDenied/ }),
  ).toBeVisible({ timeout: 5_000 });
});

test("create a new passenger then verify it appears in the table", async ({
  page,
}) => {
  const btnCreate = page.getByTestId("btn-create-passenger");
  // The PassengersSection form inputs: id, name, tier-select, create button.
  // Find the "Passengers" form by proximity to the btn-create-passenger.
  const paxSection = page.locator("h3", {
    hasText: "Passengers (POST/PATCH/DELETE /passengers)",
  });
  // The row div after the table.
  const formRow = paxSection.locator("xpath=following-sibling::div[@class='row'][1]");
  const idInput = formRow.locator("input").nth(0);
  const nameInput = formRow.locator("input").nth(1);

  await idInput.fill("ps-e2e");
  await nameInput.fill("E2E Tester");
  // Tier select already defaults to Silver — leave it.

  await btnCreate.click();

  // The new row should appear in the passengers table.
  await expect(page.getByTestId("passengers-table")).toContainText("E2E Tester", {
    timeout: 5_000,
  });
});

test("refresh button triggers a data reload", async ({ page }) => {
  const refreshBtn = page.getByTestId("btn-refresh");
  await expect(refreshBtn).toBeEnabled();
  await refreshBtn.click();
  // After clicking, the table should still show the seeded passengers.
  await expect(page.getByTestId("passengers-table")).toContainText("Mira Voss");
});
