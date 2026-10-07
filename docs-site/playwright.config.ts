import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/browser",
  outputDir: "./.browser-artifacts/test-results",
  reporter: [
    ["list"],
    ["html", { open: "never", outputFolder: ".browser-artifacts/playwright-report" }],
  ],
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  ...(process.env.CI ? { workers: 2 } : {}),
  use: { baseURL: "http://127.0.0.1:4173", trace: "retain-on-failure" },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
      testIgnore: "**/accessibility.spec.ts",
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
      testIgnore: "**/accessibility.spec.ts",
    },
  ],
  webServer: {
    // Playwright owns this process, including when Astro detects an agent.
    command: "pnpm preview --port 4173 --ignore-lock",
    url: "http://127.0.0.1:4173/",
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
