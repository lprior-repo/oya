import { test } from '@playwright/test';

test('capture all errors', async ({ page }) => {
  const errors = [];
  const logs = [];

  page.on('console', msg => {
    logs.push(`${msg.type()}: ${msg.text()}`);
    console.log(`CONSOLE [${msg.type()}]:`, msg.text());
  });

  page.on('pageerror', err => {
    errors.push(err.message);
    console.log('PAGE ERROR:', err.message);
    console.log('Stack:', err.stack);
  });

  page.on('requestfailed', request =>
    console.log('FAILED:', request.url(), request.failure()?.errorText)
  );

  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(3000);

  console.log('\n=== SUMMARY ===');
  console.log('Total console messages:', logs.length);
  console.log('Total errors:', errors.length);

  if (errors.length > 0) {
    console.log('\n=== ERRORS ===');
    errors.forEach(e => console.log(e));
  }
});
