import { expect, test } from "@playwright/test";
import { runPromise } from "./effect";
import {
  addNodeFromSidebar,
  attachPageErrorSink,
  ensureStableShell,
  nodeCount,
  openCanvasContextMenu,
  waitForEditorShell,
} from "./flow-helpers";

test("filters sidebar nodes with search", async ({ page }) => {
  const errors = attachPageErrorSink(page);
  await runPromise(waitForEditorShell(page));

  const input = page.getByPlaceholder("Search nodes...");
  await input.fill("no-node-should-match-this-query");
  await expect(page.getByText("No nodes found", { exact: true })).toBeVisible();

  await input.fill("HTTP");
  await expect(page.locator("aside button").filter({ hasText: "HTTP Trigger" }).first()).toBeVisible();

  await ensureStableShell(page, errors);
});

test("context menu opens palette and closes with escape", async ({ page }) => {
  const errors = attachPageErrorSink(page);
  await runPromise(waitForEditorShell(page));

  await openCanvasContextMenu(page);
  await page.getByRole("button", { name: "Add Node" }).evaluate((element: HTMLElement) =>
    element.click(),
  );
  await expect(page.getByText("Quick Add Node", { exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByText("Quick Add Node", { exact: true })).toHaveCount(0);

  await ensureStableShell(page, errors);
});

test("adds multiple nodes and deletes one selected node", async ({ page }) => {
  const errors = attachPageErrorSink(page);
  await runPromise(waitForEditorShell(page));

  await runPromise(addNodeFromSidebar(page));
  await runPromise(addNodeFromSidebar(page));
  await runPromise(addNodeFromSidebar(page));

  const beforeDelete = await nodeCount(page);
  expect(beforeDelete).toBeGreaterThanOrEqual(3);

  const firstNode = page.locator("div[data-node-id]").first();
  await firstNode.click();

  const selectedPanel = page.locator("aside").filter({ hasText: "Node Name" }).first();
  await selectedPanel.getByRole("button", { name: "Delete", exact: true }).click();

  await expect(page.locator("div[data-node-id]")).toHaveCount(beforeDelete - 1);
  await ensureStableShell(page, errors);
});

test("extend flow panel supports select, apply, and clear", async ({ page }) => {
  const errors = attachPageErrorSink(page);
  await runPromise(waitForEditorShell(page));

  const node = await runPromise(addNodeFromSidebar(page));
  await node.click();

  const selectedPanel = page.locator("aside").filter({ hasText: "Extend Flow" }).first();
  const selectAll = selectedPanel.getByRole("button", { name: "Select all", exact: true });
  await expect(selectAll).toBeVisible();
  await selectAll.click();

  const applySelected = selectedPanel.getByRole("button", { name: /Apply Selected/ });
  await expect(applySelected).toBeVisible();
  await applySelected.click();

  await expect(selectedPanel).toContainText(/Applied .* batch #/);

  await selectAll.click();
  await selectedPanel.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(selectedPanel.getByRole("button", { name: /Apply Selected/ })).toHaveCount(0);

  await ensureStableShell(page, errors);
});
