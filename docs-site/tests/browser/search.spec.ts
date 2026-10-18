import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("search loads on demand and finds CLI flags and Recite syntax", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  await page.goto("/");
  expect(requests.filter((url) => url.includes("/pagefind/"))).toEqual([]);
  const search = page.getByRole("button", { name: "Search", exact: true });
  await search.click();
  const dialog = page.getByRole("dialog");
  const input = dialog.getByRole("textbox", { name: "Search", exact: true });
  await expect(input).toBeFocused();
  for (const [query, title] of [
    ["--output-format", "CLI"],
    ["speaker=guard", "Source Format"],
    ["stable IDs", "Source Format"],
  ]) {
    await input.fill(query!);
    await expect(dialog.getByRole("link", { name: title!, exact: true }).first()).toBeVisible();
  }
  expect(requests.some((url) => url.includes("/pagefind/"))).toBe(true);
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa", "best-practice"])
        .analyze()
    ).violations,
  ).toEqual([]);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(search).toBeFocused();
});

test("a failed search download can be retried without losing the page", async ({ page }) => {
  await page.route("**/pagefind/pagefind-ui.js", (route) => route.abort());
  await page.goto("/");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByRole("status")).toContainText("could not load");
  await page.keyboard.press("Escape");
  await page.unroute("**/pagefind/pagefind-ui.js");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await dialog.getByRole("textbox", { name: "Search", exact: true }).fill("first scene");
  await expect(
    dialog.getByRole("link", { name: "First Scene", exact: true }).first(),
  ).toBeVisible();
});
