const { test, expect } = require('@playwright/test');

test.describe('Flow-WASM Component Parity', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:8082');
    // Wait for the shell rendered by our Dioxus components
    await page.waitForSelector('.canvas', { timeout: 15000 });
  });

  test('Given a user adds a node, When they click it, Then the settings sidebar is shown', async ({ page }) => {
    // Given the node palette is visible
    await expect(page.locator('aside >> text=Nodes')).toBeVisible();

    // When the user adds an HTTP trigger node
    await page.click('text=HTTP Trigger');
    
    // And selects that node on the canvas
    const node = page.locator('.node-card').first();
    await node.click();
    
    // Then the configuration sidebar is visible
    const sidebar = page.locator('aside >> text=Node Settings');
    await expect(sidebar).toBeVisible();
    
    // And expression controls are rendered
    await expect(page.locator('text=EXPRESSION')).toBeVisible();
  });

  test('Given a node exists, When the user deletes it, Then it is removed from the canvas', async ({ page }) => {
    await page.click('text=HTTP Trigger');
    const nodeCount = await page.locator('.node-card').count();
    
    await page.hover('.node-card');
    await page.click('text=×');
    
    await expect(page.locator('.node-card')).toHaveCount(nodeCount - 1);
  });

  test('Given sidebar search, When query has no matches, Then an empty-state message is shown', async ({ page }) => {
    const searchInput = page.locator('input[placeholder="Search nodes..."]');

    await searchInput.fill('no-node-should-match-this-query');
    await expect(page.locator('text=No nodes found')).toBeVisible();

    await searchInput.fill('HTTP');
    await expect(page.locator('button:has-text("HTTP Trigger")')).toBeVisible();
  });

  test('Given a node template with docs, When hovered, Then the tooltip provides documentation link', async ({ page }) => {
    const httpTrigger = page.locator('button:has-text("HTTP Trigger")').first();

    await httpTrigger.hover();
    await expect(page.locator('text=View documentation')).toBeVisible();
    await expect(page.locator('a[href*="/docs/10_RESTATE_SDK.md#http-trigger"]')).toHaveCount(1);
  });
});
