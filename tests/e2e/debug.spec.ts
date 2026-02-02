import { test } from '@playwright/test';

test('debug homepage', async ({ page }) => {
  // Listen for console messages
  page.on('console', msg => console.log('BROWSER CONSOLE:', msg.type(), msg.text()));

  // Listen for page errors
  page.on('pageerror', err => console.log('PAGE ERROR:', err.message));

  // Listen for failed requests
  page.on('requestfailed', request =>
    console.log('FAILED REQUEST:', request.url(), request.failure()?.errorText)
  );

  console.log('Navigating to homepage...');
  await page.goto('http://localhost:3000/');

  // Wait a bit for WASM to load
  await page.waitForTimeout(5000);

  // Take screenshot
  await page.screenshot({ path: 'debug-screenshot.png', fullPage: true });

  // Get HTML content
  const html = await page.content();
  console.log('Page HTML length:', html.length);

  // Check what's in the #app div
  const appContent = await page.locator('#app').innerHTML();
  console.log('App div content:', appContent);

  // Check if any visible text
  const bodyText = await page.locator('body').textContent();
  console.log('Body text:', bodyText);
});
