import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

for (const theme of ["light", "dark"] as const) {
  test(`generated pages pass axe in ${theme}`, async ({ page }, testInfo) => {
    test.setTimeout(120_000);
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await page.goto("/");
    const sitemap = await (await page.request.get("/sitemap-0.xml")).text();
    const paths = await page.evaluate(
      (xml) =>
        Array.from(
          new DOMParser().parseFromString(xml, "application/xml").querySelectorAll("url > loc"),
        )
          .map((node) => new URL(node.textContent!, location.href).pathname),
      sitemap,
    );
    expect(paths.length).toBeGreaterThan(1);
    for (const path of paths) {
      await page.goto(path);
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
      // Every overflowing block must be reachable for keyboard scrolling.
      await expect
        .poll(() =>
          page
            .locator("pre, table")
            .evaluateAll((blocks) =>
              blocks.every(
                (block) => block.scrollWidth <= block.clientWidth || block.tabIndex === 0,
              )
            )
        )
        .toBe(true);
      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"])
        .analyze();
      const pageName = path === "/" ? "home" : path.slice(1, -1).replaceAll("/", "-");
      await testInfo.attach(`axe-${theme}-${pageName}`, {
        body: JSON.stringify(results, null, 2),
        contentType: "application/json",
      });
      expect(results.violations).toEqual([]);
      // Same foreground/background colors can appear as incomplete, not violations.
      expect(results.incomplete.filter(result => result.id === "color-contrast")).toEqual([]);
    }
  });
}

test("dark static pages pass axe without site scripts", async ({ browser }) => {
  const context = await browser.newContext({ colorScheme: "dark" });
  try {
    await context.route(
      "**/*",
      (route) => route.request().resourceType() === "script" ? route.abort() : route.continue(),
    );
    const page = await context.newPage();
    await page.goto("/getting-started/first-scene/");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("First Scene");
    // Axe injects its audit library inline; external site scripts stay blocked.
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"])
      .analyze();
    expect(results.violations).toEqual([]);
  } finally {
    await context.close();
  }
});

test("playground prompts and compiler diagnostics pass axe", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.goto("/");
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  for (let step = 0; step < 3; step += 1) {
    await page.getByRole("button", { name: "Continue", exact: true }).click();
  }
  await expect(page.getByRole("status")).toHaveText("Choose a reply.");
  expect(
    (await new AxeBuilder({ page }).withTags([
      "wcag2a",
      "wcag2aa",
      "wcag21aa",
      "wcag22aa",
      "best-practice",
    ]).analyze()).violations,
  ).toEqual([]);
  await page.getByRole("textbox", { name: "Recite source" }).fill(":: start default\n-> missing\n");
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("could not compile");
  expect(
    (await new AxeBuilder({ page }).withTags([
      "wcag2a",
      "wcag2aa",
      "wcag21aa",
      "wcag22aa",
      "best-practice",
    ]).analyze()).violations,
  ).toEqual([]);
});
