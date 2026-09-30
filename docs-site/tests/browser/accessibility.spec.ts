import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

for (const theme of ["light", "dark"] as const) {
  test(`landing and guide pages pass axe in ${theme}`, async ({ page }, testInfo) => {
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    for (const path of ["/", "/getting-started/first-scene/", "/guides/alternatives/"]) {
      await page.goto(path);
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
      // Expressive Code marks only genuinely scrollable blocks after ResizeObserver settles.
      await expect
        .poll(() =>
          page
            .locator(".expressive-code pre")
            .evaluateAll((blocks) =>
              blocks.every(
                (block) => block.scrollWidth <= block.clientWidth || block.tabIndex === 0,
              ),
            ),
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
    }
  });
}
