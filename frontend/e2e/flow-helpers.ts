import { expect, type Locator, type Page } from "@playwright/test";
import { fromPromise, runPromise, succeed, tap, type Effect } from "./effect";

export const attachPageErrorSink = (page: Page): string[] => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  return errors;
};

export const waitForEditorShell = (page: Page): Effect<Page> =>
  tap(succeed(page), async () => {
    await page.goto("/");
    await expect(page.locator("main")).toHaveCount(1);
    await expect(page.locator("aside").first()).toBeVisible();
    await expect(page.getByPlaceholder("Search nodes...")).toBeVisible();
  });

export const addNodeFromSidebar = (
  page: Page,
  label = "HTTP Trigger",
): Effect<Locator> =>
  tap(succeed(page.getByTestId("flow-node").last()), async (node) => {
    const button = page.locator("aside button").filter({ hasText: label }).first();
    await expect(button).toBeVisible();
    await button.evaluate((element: HTMLElement) => element.click());
    await expect(node).toBeVisible();
  });

export const nodeCount = async (page: Page): Promise<number> => {
  return page.locator("div[data-node-id]").count();
};

export const flowNode = (page: Page): Locator => page.getByTestId("flow-node");

export const openCanvasContextMenu = async (page: Page): Promise<void> => {
  const canvas = page.locator("main");
  const rect = await canvas.boundingBox();
  expect(rect).not.toBeNull();
  if (!rect) {
    return;
  }

  await canvas.click({
    button: "right",
    position: {
      x: Math.max(20, Math.floor(rect.width * 0.5)),
      y: Math.max(20, Math.floor(rect.height * 0.5)),
    },
  });
  await expect(page.getByRole("button", { name: "Add Node" })).toBeVisible();
};

export const assertNoPageErrors = (errors: string[]): Effect<void> =>
  fromPromise(async () => {
    expect(errors).toEqual([]);
  });

export const assertGraphIntegrity = (page: Page): Effect<void> =>
  fromPromise(async () => {
    const graph = await page.evaluate(() => {
      const nodes = Array.from(document.querySelectorAll("div[data-node-id]")).map((node) =>
        node.getAttribute("data-node-id"),
      );
      const unique = new Set(nodes.filter((value): value is string => typeof value === "string"));
      return { count: nodes.length, uniqueCount: unique.size };
    });

    expect(graph.count).toBe(graph.uniqueCount);
    expect(graph.count).toBeLessThanOrEqual(80);
  });

export const ensureStableShell = async (page: Page, errors: string[]): Promise<void> => {
  await expect(page.locator("main")).toHaveCount(1);
  await expect(page.locator("aside").first()).toBeVisible();
  await runPromise(assertNoPageErrors(errors));
  await runPromise(assertGraphIntegrity(page));
};
