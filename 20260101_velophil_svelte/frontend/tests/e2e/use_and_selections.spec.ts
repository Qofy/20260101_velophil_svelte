import { test, expect } from '@playwright/test';

// Helpers
async function openApp(page) {
  await page.goto('/');
  // Wait for scene to be ready by waiting for menu buttons to appear
  await page.getByTestId('btn-settings').waitFor();
}

test.describe('Custom USE mode and Selections', () => {
  test('USE mode allows click (flip) but disables drag', async ({ page }) => {
    await openApp(page);
    // Switch to CUSTOM and enable USE mode
    await page.getByRole('button', { name: 'CUSTOM' }).click({ force: true });
    await page.getByRole('button', { name: 'USE' }).click({ force: true });

    const card = page.locator('.element').first();
    await card.waitFor();

    // Click to flip in USE mode
    await card.click();
    await expect(card).toHaveClass(/flipped/);

    // Try to drag; should not get dragging class in USE mode
    const box = await card.boundingBox();
    if (!box) throw new Error('No card bbox');
    const start = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    await page.mouse.move(start.x, start.y);
    await page.mouse.down();
    await page.mouse.move(start.x + 30, start.y + 30);
    await page.mouse.up();
    await expect(card).not.toHaveClass(/dragging/);
  });

  test('Save/Load named selections', async ({ page }) => {
    await openApp(page);

    // Enter selection mode
    await page.getByRole('button', { name: 'SELECT' }).click({ force: true });

    const card0 = page.locator('.element').nth(0);
    const card1 = page.locator('.element').nth(1);
    await card0.waitFor();
    await card1.waitFor();

    // Click two cards to select
    await card0.click();
    await card1.click();
    await expect(card0).toHaveClass(/selected/);
    await expect(card1).toHaveClass(/selected/);

    // Name + SAVE
    const name = 'sel-e2e-' + Math.floor(Math.random() * 10000);
    await page.getByPlaceholder('Selection Name').fill(name);
    await page.getByRole('button', { name: 'SAVE' }).click();

    // Clear then LOAD via overlay
    await page.getByRole('button', { name: 'Clear' }).click();
    await expect(card0).not.toHaveClass(/selected/);
    await expect(card1).not.toHaveClass(/selected/);

    await page.getByRole('button', { name: 'LOAD' }).click();
    // Select the saved name in overlay
    await page.getByRole('button', { name }).click();

    // After load, selection should be re-applied
    await expect(card0).toHaveClass(/selected/);
    await expect(card1).toHaveClass(/selected/);
  });
});

test.describe('Wordl play-in-place (no card zoom)', () => {
  test('Starting Wordl does not scale the card', async ({ page }) => {
    await openApp(page);

    // Open settings and enable Wordl button
    await page.getByTestId('btn-settings').click({ force: true });
    // Toggle show Wordl button (checkbox label)
    await page.getByText('Show button Play Wordle').locator('xpath=..').getByRole('checkbox').check();
    // Close settings
    await page.getByRole('button', { name: 'Close' }).click();

    const card = page.locator('.element').first();
    await card.waitFor();

    // Flip the card so back is visible
    await card.click();

    // Record bounding box before starting Wordl
    const before = await card.boundingBox();
    if (!before) throw new Error('No bbox before');

    // Start Wordl via overlay prompt area
    const overlay = card.locator('.wordl-overlay-play');
    await overlay.waitFor();
    await overlay.click();

    // Wait a bit for any UI updates
    await page.waitForTimeout(300);

    // Bounding box should remain approximately the same (no fullscreen zoom)
    const after = await card.boundingBox();
    if (!after) throw new Error('No bbox after');
    const areaBefore = before.width * before.height;
    const areaAfter = after.width * after.height;
    const ratio = areaAfter / areaBefore;
    expect(ratio).toBeGreaterThan(0.85);
    expect(ratio).toBeLessThan(1.15);
  });
});
