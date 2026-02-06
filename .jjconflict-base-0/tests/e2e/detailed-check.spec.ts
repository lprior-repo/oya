import { test, expect } from '@playwright/test';

test('detailed render check', async ({ page }) => {
  // Enable console logging
  page.on('console', msg => console.log(`[BROWSER ${msg.type()}]:`, msg.text()));

  // Catch any errors
  page.on('pageerror', error => console.log(`[PAGE ERROR]:`, error.message));

  await page.goto('http://localhost:3000/');

  // Wait for app to hydrate/mount
  await page.waitForTimeout(2000);

  // Check body content
  const bodyText = await page.locator('body').textContent();
  console.log('Body text length:', bodyText?.length);
  console.log('Body text preview:', bodyText?.substring(0, 200));

  // Check for specific elements
  const h1Count = await page.locator('h1').count();
  console.log('H1 count:', h1Count);

  if (h1Count > 0) {
    const h1Texts = await page.locator('h1').allTextContents();
    console.log('H1 texts:', h1Texts);
  }

  // Check for navigation
  const navLinks = await page.locator('nav a, .app-nav a').count();
  console.log('Nav links:', navLinks);

  // Check for main content
  const mainContent = await page.locator('main, .app-main').count();
  console.log('Main content areas:', mainContent);

  // Take screenshot for visual verification
  await page.screenshot({ path: 'detailed-check.png', fullPage: true });
  console.log('Screenshot saved to detailed-check.png');

  // Assertions
  expect(h1Count).toBeGreaterThan(0);
  expect(bodyText?.length).toBeGreaterThan(10);
});
