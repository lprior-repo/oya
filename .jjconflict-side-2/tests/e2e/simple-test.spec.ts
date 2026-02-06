import { test, expect } from '@playwright/test';

test('check if Leptos mounted', async ({ page }) => {
  await page.goto('http://localhost:3000/');

  // Wait for page to load
  await page.waitForLoadState('networkidle');

  // Take screenshot
  await page.screenshot({ path: 'homepage.png', fullPage: true });

  // Check if .app-container exists (Leptos should render this)
  const appContainer = await page.locator('.app-container').count();
  console.log('app-container count:', appContainer);

  // Check if there's an h1 with OYA text
  const h1Count = await page.locator('h1').count();
  console.log('h1 count:', h1Count);

  if (h1Count > 0) {
    const h1Text = await page.locator('h1').first().textContent();
    console.log('First h1 text:', h1Text);
  }

  // Get all visible text
  const allText = await page.locator('body').allTextContents();
  console.log('All body text:', allText);
});
