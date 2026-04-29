import { expect, test } from "@playwright/test";
import { runPromise } from "./effect";
import { addNodeFromSidebar, attachPageErrorSink, waitForEditorShell } from "./flow-helpers";

test("loads core editor shell", async ({ page }) => {
  const errors = attachPageErrorSink(page);

  await runPromise(waitForEditorShell(page));

  await expect(page.locator("main")).toHaveCount(1);
  await expect(page.getByText("Application Composer")).toBeVisible();
  expect(errors).toEqual([]);
});

test("adds and selects a node", async ({ page }) => {
  await runPromise(waitForEditorShell(page));
  const node = await runPromise(addNodeFromSidebar(page));

  await node.click();
  const selectedPanel = page.locator("aside").filter({ hasText: "Node Name" }).first();
  await expect(selectedPanel.getByRole("button", { name: "Delete", exact: true })).toBeVisible();
});

test("dragging node keeps it visible", async ({ page }) => {
  await runPromise(waitForEditorShell(page));
  const node = await runPromise(addNodeFromSidebar(page));

  const start = await node.boundingBox();
  expect(start).not.toBeNull();
  if (!start) {
    return;
  }

  await page.mouse.move(start.x + start.width / 2, start.y + start.height / 2);
  await page.mouse.down();
  await page.mouse.move(start.x + start.width / 2 + 80, start.y + start.height / 2 + 40);
  await page.mouse.up();

  await expect(node).toBeVisible();
  const end = await node.boundingBox();
  expect(end).not.toBeNull();
});
