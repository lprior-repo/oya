const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  console.log('Navigating to http://192.168.150.177:8083...');
  try {
    await page.goto('http://192.168.150.177:8083', { waitUntil: 'networkidle', timeout: 60000 });
    console.log('Page loaded.');
    
    // Wait for the rebuild toast to disappear
    console.log('Waiting for rebuild toast to disappear...');
    await page.waitForSelector('#__dx-toast', { state: 'hidden', timeout: 30000 });
    console.log('Rebuild toast hidden.');
    
    // Wait for the app to hydrate
    console.log('Waiting for "Restate Invocations" text...');
    await page.waitForSelector('text=Restate Invocations', { timeout: 30000 });
    console.log('Found "Restate Invocations"!');
    
    const sidebarText = await page.textContent('aside');
    console.log('Sidebar Text:', sidebarText);
    
    await page.screenshot({ path: 'playwright_proof.png', fullPage: true });
    console.log('Screenshot saved to playwright_proof.png');
    
  } catch (err) {
    console.error('Error:', err);
    await page.screenshot({ path: 'error_proof.png', fullPage: true });
    const content = await page.content();
    console.log('Page Content:', content);
  } finally {
    await browser.close();
  }
})();
