import { test, expect } from '@playwright/test';

test('OYA homepage fully loads', async ({ page }) => {
  page.on('console', msg => console.log(`[${msg.type()}]:`, msg.text()));
  page.on('pageerror', err => console.log('ERROR:', err.message));

  await page.goto('http://localhost:3000/');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(3000);

  // Take screenshot
  await page.screenshot({ path: 'final-test.png', fullPage: true });

  // Check if Leptos app mounted
  const appContainerCount = await page.locator('.app-container').count();
  console.log('✓ app-container elements:', appContainerCount);

  const h1Count = await page.locator('h1').count();
  console.log('✓ h1 elements:', h1Count);

  if (h1Count > 0) {
    const h1Text = await page.locator('h1').first().textContent();
    console.log('✓ First h1 text:', h1Text);
  }

  // Verify Leptos rendered
  await expect(page.locator('.app-container')).toBeVisible();
  await expect(page.locator('h1').first()).toContainText('OYA');
});
