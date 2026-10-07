import { expect, test } from "@playwright/test";

test("landing page leads into the manual", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("A dialogue language for games");
  await expect(page.getByRole("heading", { level: 2, name: "At the junction" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Recite source" })).toContainText(
    "N-2 is marked as a cable shaft.",
  );
  await page.getByRole("link", { name: "Write a first scene" }).click();
  await expect(page).toHaveURL(/\/getting-started\/first-scene\/$/);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("First Scene");
});

test("landing page offers a bounded comparison of dialogue tools", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Choosing dialogue tools" }).click();
  await expect(page).toHaveURL(/\/guides\/alternatives\/$/);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Choosing dialogue tools");
  await expect(page.getByText("qualitative assessments", { exact: false })).toBeVisible();
});

test("landing page reflows and records visual evidence", async ({ page }, testInfo) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  for (const width of [320, 390, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await expect(page.locator(".scene-script")).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
    expect(overflow, `horizontal overflow at ${width}px`).toBeLessThanOrEqual(1);
    if (width !== 768) {
      await testInfo.attach(`landing-${width}`, {
        body: await page.screenshot({ fullPage: true }),
        contentType: "image/png",
      });
    }
  }
});

test("alternatives guide remains readable on a narrow screen", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto("/guides/alternatives/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Choosing dialogue tools");
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  await testInfo.attach("alternatives-390", {
    body: await page.screenshot({ fullPage: true }),
    contentType: "image/png",
  });
});

test("source and manual remain available without JavaScript", async ({ browser }) => {
  const colors = new Set<string>();
  for (const colorScheme of ["light", "dark"] as const) {
    const context = await browser.newContext({
      javaScriptEnabled: false,
      colorScheme,
      viewport: { width: 390, height: 900 },
    });
    try {
      const page = await context.newPage();
      await page.goto("/");
      await expect(page.getByRole("textbox", { name: "Recite source" })).toContainText(
        ":: junction default",
      );
      await expect(page.getByRole("button", { name: "Run scene" })).toBeHidden();
      await expect(page.getByRole("combobox", { name: "Select theme", exact: true })).toBeHidden();
      colors.add(await page.locator("body").evaluate(body => getComputedStyle(body).color));
      await page.getByRole("button", { name: "Site menu", exact: true }).click();
      await expect(page.locator("#global-menu")).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(page.locator("#global-menu")).toBeHidden();
      await page.getByRole("link", { name: "Read the source format" }).click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText("Source Format");
    } finally {
      await context.close();
    }
  }
  expect(colors.size).toBe(2);
});

test("long manual code blocks can be scrolled with a keyboard", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto("/getting-started/first-scene/");
  const blocks = page.locator("pre");
  const index = await blocks.evaluateAll((items) =>
    items.findIndex((item) => item.scrollWidth > item.clientWidth)
  );
  expect(index).toBeGreaterThanOrEqual(0);
  const block = blocks.nth(index);
  await expect(block).toHaveAttribute("tabindex", "0");
  await block.focus();
  await expect(block).toBeFocused();
  const before = await block.evaluate((item) => item.scrollLeft);
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => block.evaluate((item) => item.scrollLeft)).toBeGreaterThan(before);
});

test("wide manual tables can be scrolled with a keyboard", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto("/guides/distribution/");
  const tables = page.locator("table");
  // Exercise genuinely wide authored content independently of platform fonts.
  await tables.first().evaluate(table => {
    for (const row of table.querySelectorAll("tr")) {
      for (let column = 0; column < 24; column += 1) {
        const cell = document.createElement(row.closest("thead") ? "th" : "td");
        cell.textContent = `column-${column}`;
        row.append(cell);
      }
    }
  });
  const index = await tables.evaluateAll((items) =>
    items.findIndex((item) => item.scrollWidth > item.clientWidth)
  );
  expect(index).toBeGreaterThanOrEqual(0);
  const table = tables.nth(index);
  await table.focus();
  await expect(table).toBeFocused();
  const before = await table.evaluate((item) => item.scrollLeft);
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => table.evaluate((item) => item.scrollLeft)).toBeGreaterThan(before);
});

test("showcase is honest about adoption and excludes investigation fixtures and drafts", async ({ page }) => {
  await page.goto("/showcase/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Projects using Recite");
  await expect(page.getByText("No projects are featured here yet.", { exact: true })).toBeVisible();
  for (
    const route of [
      "/showcase/junction/",
      "/about/",
      "/learn/first-scene/",
      "/release-notes/",
      "/reference/schema/",
      "/adapters/authoring-refresh/",
    ]
  ) {
    expect((await page.request.get(route)).status(), route).toBe(404);
  }
});
