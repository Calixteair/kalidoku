import { test, expect } from "@playwright/test";

const BASE_URL = process.env.KD_BASE_URL ?? "https://kalidoku.calixteair.fr";

test.describe("real play flow", () => {
  test.skip(
    !process.env.KD_BASE_URL && !process.env.CI,
    "no live API — set KD_BASE_URL to run against a real backend",
  );

  test("daily grid → cell → autocomplete → submit → end state", async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 720 });
    await page.goto(BASE_URL);

    // Wait for the grid to render. The Play button is shown until we've started.
    const playButton = page.getByRole("button", { name: /jouer|play/i }).first();
    await expect(playButton).toBeVisible({ timeout: 15_000 });
    await playButton.click();

    // Click the first empty cell. CellButton renders <button> with cell labels.
    const firstCell = page
      .getByRole("button", { name: /case|cell/i })
      .filter({ hasNot: page.locator("[data-filled='true']") })
      .first();
    await expect(firstCell).toBeVisible({ timeout: 10_000 });
    await firstCell.click();

    // Autocomplete dialog opens.
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();

    const input = dialog.getByRole("combobox");
    await input.fill("châtelet");

    // Select the first suggestion (option role).
    const firstOption = dialog.getByRole("option").first();
    await expect(firstOption).toBeVisible({ timeout: 10_000 });
    await firstOption.click();

    // After submit, dialog closes and either next cell can be tapped or end-of-game shows.
    await expect(dialog).toBeHidden({ timeout: 10_000 });

    // End state appears as either a score readout or the see-solutions button.
    const scoreReadout = page.getByText(/score/i).first();
    await expect(scoreReadout).toBeVisible({ timeout: 10_000 });
  });
});
