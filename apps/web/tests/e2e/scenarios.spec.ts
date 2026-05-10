import { expect, test } from "@playwright/test";

test("portfolio page loads and shows stats", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("Portfolio").first()).toBeVisible();
});

test("scenarios page runs an in-browser simulation", async ({ page }) => {
  await page.goto("/scenarios");
  await page.getByTestId("run-simulation").click();
  await expect(page.getByTestId("metrics")).toBeVisible({ timeout: 30_000 });
});
