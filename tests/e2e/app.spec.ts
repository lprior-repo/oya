import { test, expect } from '@playwright/test';

test.describe('OYA Application', () => {
  test('homepage loads successfully', async ({ page }) => {
    await page.goto('/');

    // Check that the page title is correct
    await expect(page).toHaveTitle(/OYA Graph Visualization/);

    // Wait for WASM to load and Leptos to mount
    await page.waitForSelector('.app-container', { timeout: 10000 });

    // Verify the app rendered
    await expect(page.locator('h1').first()).toContainText('OYA');
  });

  test('API health endpoint responds', async ({ request }) => {
    const response = await request.get('/api/health');
    expect(response.ok()).toBeTruthy();
  });

  test('dashboard route works', async ({ page }) => {
    await page.goto('/dashboard');

    // Verify we're on the dashboard
    await expect(page).toHaveURL(/.*dashboard/);
  });

  test('tasks route works', async ({ page }) => {
    await page.goto('/tasks');

    // Verify we're on the tasks page
    await expect(page).toHaveURL(/.*tasks/);
  });

  test('beads route works', async ({ page }) => {
    await page.goto('/beads');

    // Verify we're on the beads page
    await expect(page).toHaveURL(/.*beads/);
  });
});
