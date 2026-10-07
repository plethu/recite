import { expect, test } from "@playwright/test";

test("skip link and mobile controls work with a keyboard", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto("/getting-started/first-scene/");
  await page.keyboard.press("Tab");
  await expect(page.getByRole("link", { name: "Skip to content" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#content")).toBeFocused();

  await page.goto("about:blank");
  await page.goto("/getting-started/first-scene/");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  const menu = page.getByRole("checkbox", { name: "Menu", exact: true });
  await expect(menu).toBeFocused();
  await page.keyboard.press("Space");
  await expect(menu).toBeChecked();
  await expect(page.locator("#book-search-input")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.locator(".book-menu nav ul a").first()).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(menu).not.toBeChecked();
  await expect(menu).toBeFocused();

  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  const contents = page.getByRole("checkbox", { name: "Table of Contents" });
  await expect(contents).toBeFocused();
  await page.keyboard.press("Space");
  await expect(contents).toBeChecked();
  await expect(page.locator("#TableOfContents")).toBeVisible();
});

test("search results lead to the manual", async ({ page }) => {
  await page.goto("/");
  await page.locator("#book-search-input").fill("first scene");
  const result = page.locator("#book-search-results").getByRole("link", {
    name: "First Scene",
    exact: true,
  });
  await expect(result).toBeVisible();
  await result.focus();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/getting-started\/first-scene\/$/);
});

test("manual theme persists and system theme follows the OS", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  const theme = page.getByRole("combobox", { name: "Theme", exact: true });
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
