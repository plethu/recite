import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    testTimeout: 30_000,
    hookTimeout: 30_000,
    projects: [
      "docs-site/vitest.config.ts",
      "editors/vscode/vitest.config.ts",
      {
        test: {
          name: "tooling",
          include: ["editors/recite-neovim/test/**/*.test.mjs"],
        },
      },
    ],
  },
});
