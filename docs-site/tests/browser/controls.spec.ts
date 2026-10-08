import { expect, test } from "@playwright/test";

test("skip link and both mobile menus work with a keyboard", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 900 });
  await page.goto("/reference/source-format/");
  await page.keyboard.press("Tab");
  await expect(page.getByRole("link", { name: "Skip to content" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#_top")).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth))
    .toBeLessThanOrEqual(1);
  const menuBox = await page.getByRole("button", { name: "Site menu", exact: true }).boundingBox();
  const headerBox = await page.getByRole("banner").boundingBox();
  expect(menuBox).not.toBeNull();
  expect(headerBox).not.toBeNull();
  expect(menuBox!.y + menuBox!.height).toBeLessThanOrEqual(headerBox!.y + headerBox!.height + 1);

  const siteMenu = page.getByRole("button", { name: "Site menu", exact: true });
  await siteMenu.focus();
  await page.keyboard.press("Enter");
  const links = page.locator("#global-menu").getByRole("link");
  await expect(links.first()).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(links.first()).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.locator("#global-menu")).toBeHidden();
  await expect(siteMenu).toBeFocused();

  const referenceMenu = page.getByRole("button", { name: "Reference menu", exact: true });
  await referenceMenu.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("#starlight__sidebar")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("#starlight__sidebar")).toBeHidden();
  await expect(referenceMenu).toBeFocused();
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect(page.locator("#starlight__sidebar")).toBeVisible();
});

test("search recovers after a miss and leads to the manual", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  const dialog = page.getByRole("dialog");
  const input = dialog.getByRole("textbox", { name: "Search", exact: true });
  const result = dialog.getByRole("link", { name: "First Scene", exact: true }).first();
  await input.fill("first scene");
  await expect(result).toBeVisible();
  await input.fill("qzxvbnmplkjhgfdsa987654");
  await expect(dialog.locator(".pagefind-ui__result-link")).toHaveCount(0);
  await input.fill("first scene");
  await expect(result).toBeVisible();
  await result.focus();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/getting-started\/first-scene\/$/);
  // Full navigation must initialize a fresh controller when returning home.
  await page.getByRole("link", { name: "Recite home", exact: true }).click();
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(page.getByRole("list", { name: "Dialogue transcript" })).toContainText(
    "N-2 is stamped",
  );
});

test("manual theme persists and system theme follows the OS", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  const theme = page.getByRole("combobox", { name: "Select theme", exact: true });
  await theme.selectOption("dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.reload();
  await expect(theme).toHaveValue("dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await theme.selectOption("auto");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});
