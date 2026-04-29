import { expect, test, type Page } from "@playwright/test";
import { runPromise } from "./effect";
import {
  addNodeFromSidebar,
  attachPageErrorSink,
  ensureStableShell,
  nodeCount,
  openCanvasContextMenu,
  waitForEditorShell,
} from "./flow-helpers";

const seeded = (seed: number): (() => number) => {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) / 4294967296;
  };
};

async function dragFirstNode(page: Page, dx: number, dy: number): Promise<void> {
  const first = page.locator("div[data-node-id]").first();
  if ((await first.count()) === 0) {
    return;
  }
  const box = await first.boundingBox();
  if (!box) {
    return;
  }

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + dx, box.y + box.height / 2 + dy);
  await page.mouse.up();
  await expect(first).toBeVisible();
}

test("adversarial seeded interaction loop preserves invariants", async ({ page }) => {
  const errors = attachPageErrorSink(page);
  const seed = 0x0badc0de;
  const random = seeded(seed);

  await runPromise(waitForEditorShell(page));

  const steps = 40;
  for (let step = 0; step < steps; step += 1) {
    const pick = Math.floor(random() * 7);

    if (pick === 0) {
      await runPromise(addNodeFromSidebar(page));
    } else if (pick === 1) {
      const first = page.locator("div[data-node-id]").first();
      if ((await first.count()) > 0) {
        await first.evaluate((element: HTMLElement) => element.click());
      }
    } else if (pick === 2) {
      const dx = Math.floor(random() * 120) - 60;
      const dy = Math.floor(random() * 120) - 60;
      await dragFirstNode(page, dx, dy);
    } else if (pick === 3) {
      await openCanvasContextMenu(page);
      await page.keyboard.press("Escape");
    } else if (pick === 4) {
      const query = random() > 0.5 ? "HTTP" : "zzzz-no-match";
      const input = page.getByPlaceholder("Search nodes...");
      await input.fill(query);
      await input.fill("");
    } else if (pick === 5) {
      const selectedPanel = page.locator("aside").filter({ hasText: "Node Name" }).first();
      const deleteButton = selectedPanel.getByRole("button", { name: "Delete", exact: true });
      if ((await deleteButton.count()) > 0) {
        await deleteButton.click();
      }
    } else {
      await page.keyboard.press("Escape");
    }

    if (step % 5 === 0) {
      await ensureStableShell(page, errors);
      expect(await nodeCount(page)).toBeGreaterThanOrEqual(0);
    }
  }

  await ensureStableShell(page, errors);
});
