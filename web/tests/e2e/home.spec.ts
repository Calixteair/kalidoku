import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("home", () => {
  test("renders site title and rules link at 360px", async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 720 });
    await page.goto("/");
    await expect(page.getByRole("link", { name: /kalidoku/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /règles|rules/i })).toBeVisible();

    // No horizontal scroll at 360 px.
    const overflow = await page.evaluate(() => {
      return document.documentElement.scrollWidth - document.documentElement.clientWidth;
    });
    expect(overflow).toBeLessThanOrEqual(1);
  });

  test("opens rules modal", async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 720 });
    await page.goto("/");
    await page.getByRole("button", { name: /règles|rules/i }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
  });

  test("accessibility — no axe violations on home", async ({ page }) => {
    await page.goto("/");
    const results = await new AxeBuilder({ page }).disableRules(["region"]).analyze();
    expect(results.violations).toEqual([]);
  });
});
