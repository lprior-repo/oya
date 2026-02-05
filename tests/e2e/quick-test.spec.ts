import { test } from '@playwright/test';

test('quick check', async ({ page }) => {
  page.on('console', msg => console.log(`[${msg.type()}]:`, msg.text()));

  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(5000);

  const h1s = await page.locator('h1').count();
  console.log('H1 count:', h1s);

  if (h1s > 0) {
    const text = await page.locator('h1').first().textContent();
    console.log('H1 text:', text);
  }

  await page.screenshot({ path: 'quick.png' });
});
