import { test } from '@playwright/test';

test('test direct WASM load', async ({ page }) => {
  page.on('console', msg => console.log(`[${msg.type()}]:`, msg.text()));
  page.on('pageerror', err => console.log('ERROR:', err.message));

  await page.goto('http://localhost:3000/test.html');
  await page.waitForTimeout(5000);

  const status = await page.locator('#status').innerHTML();
  console.log('Status div content:', status);

  const appDiv = await page.locator('#app').innerHTML();
  console.log('App div content:', appDiv);

  await page.screenshot({ path: 'test-direct.png' });
});
